//! Segment matching shared by discovery and real HTTP contract assertions.
use super::*;
use std::collections::HashSet;

// Intersection of segment globs. '*' never crosses '/', so a collection call
// cannot accidentally require every nested resource beneath it.
pub fn intersects(a: &str, b: &str) -> bool {
    fn go(a: &[u8], b: &[u8], i: usize, j: usize, seen: &mut HashSet<(usize, usize)>) -> bool {
        if !seen.insert((i, j)) {
            return false;
        }
        if i == a.len() && j == b.len() {
            return true;
        }
        // '#' is a nonempty numeric identifier. It keeps a typed id lookup
        // from accidentally selecting a reserved sibling such as /index_status.
        match (a.get(i), b.get(j)) {
            (Some(b'#'), Some(b'#')) => return go(a, b, i + 1, j + 1, seen),
            (Some(b'#'), Some(b'*')) => {
                return go(a, b, i + 1, j, seen) || go(a, b, i, j + 1, seen);
            }
            (Some(b'*'), Some(b'#')) => {
                return go(a, b, i, j + 1, seen) || go(a, b, i + 1, j, seen);
            }
            (Some(b'#'), Some(d)) if d.is_ascii_digit() => {
                return go(a, b, i + 1, j + 1, seen) || go(a, b, i, j + 1, seen);
            }
            (Some(d), Some(b'#')) if d.is_ascii_digit() => {
                return go(a, b, i + 1, j + 1, seen) || go(a, b, i + 1, j, seen);
            }
            (Some(b'#'), _) | (_, Some(b'#')) => return false,
            _ => (),
        }
        if a.get(i) == Some(&b'*') {
            return go(a, b, i + 1, j, seen)
                || (j < b.len() && b[j] != b'/' && go(a, b, i, j + 1, seen));
        }
        if b.get(j) == Some(&b'*') {
            return go(a, b, i, j + 1, seen)
                || (i < a.len() && a[i] != b'/' && go(a, b, i + 1, j, seen));
        }
        i < a.len() && j < b.len() && a[i] == b[j] && go(a, b, i + 1, j + 1, seen)
    }
    go(a.as_bytes(), b.as_bytes(), 0, 0, &mut HashSet::new())
}
pub fn route_pattern(path: &str) -> String {
    let mut out = String::new();
    let mut param = false;
    for c in path.chars() {
        if c == '{' {
            param = true;
            out.push('*');
        } else if c == '}' {
            param = false;
        } else if !param {
            out.push(c);
        }
    }
    out
}

/// Compile path patterns once per source file, not once per HTTP call.
pub(super) struct RouteIndex<'a> {
    patterns: Vec<(&'a Operation, String)>,
}
impl<'a> RouteIndex<'a> {
    pub fn new(declared: &'a BTreeMap<Operation, Value>) -> Self {
        Self {
            patterns: declared
                .keys()
                .map(|op| (op, route_pattern(&op.path)))
                .collect(),
        }
    }
    pub fn require(&self, method: &str, path: &str) -> Result<Vec<Operation>> {
        let found: Vec<_> = self
            .patterns
            .iter()
            .filter(|(op, pattern)| op.method == method && intersects(path, pattern))
            .map(|(op, _)| (*op).clone())
            .collect();
        anyhow::ensure!(
            !found.is_empty(),
            "undocumented consumer operation: {method} {path}"
        );
        Ok(found)
    }
}
