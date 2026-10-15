// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Structural diff between two use case documents (issue #401).
//!
//! Elements are matched by id, so a step that moved is *moved*, not removed
//! and added again; a branch whose origin changed is *rebranched*. Text
//! changes carry a word-level diff.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::Serialize;
use similar::{ChangeTag, TextDiff};

use super::{BranchEnd, ElementKind, UseCaseDocument, labels};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Change {
    Added,
    Removed,
    Edited,
    Moved,
    Rebranched,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffOp {
    Equal,
    Delete,
    Insert,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TextChunk {
    pub op: DiffOp,
    pub text: String,
}

/// Where an element is, and what it says, on one side of the diff.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Side {
    /// `preconditions`, `main`, `postconditions`, `branch` (a branch step)
    /// or `branches` (a branch itself).
    pub list: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    pub label: Option<String>,
    /// The element's text, or the branch's title.
    pub text: String,
    /// Branches only: the origin step and the end.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_step: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<BranchEnd>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<super::BranchKind>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ElementDiff {
    pub id: String,
    pub kind: ElementKind,
    pub changes: Vec<Change>,
    pub old: Option<Side>,
    pub new: Option<Side>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_diff: Option<Vec<TextChunk>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Summary {
    pub added: usize,
    pub removed: usize,
    pub edited: usize,
    pub moved: usize,
    pub rebranched: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FieldChange {
    pub old: serde_json::Value,
    pub new: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DocumentDiff {
    pub summary: Summary,
    /// Changed header fields: `title`, `primary_actor`, `secondary_actors`, `summary`.
    pub header: BTreeMap<&'static str, FieldChange>,
    /// Changed elements in the order of the new document, then removed ones.
    /// With `include_unchanged`, every element.
    pub elements: Vec<ElementDiff>,
}

/// Words, runs of whitespace and single punctuation marks, so that
/// "format." and "format" share the word.
fn tokens(text: &str) -> Vec<&str> {
    static TOKEN: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"\w+|\s+|[^\w\s]").unwrap());
    TOKEN.find_iter(text).map(|m| m.as_str()).collect()
}

/// A word-level diff of two texts, with runs of the same kind merged.
pub fn word_diff(old: &str, new: &str) -> Vec<TextChunk> {
    let (old, new) = (tokens(old), tokens(new));
    let diff = TextDiff::configure()
        .algorithm(similar::Algorithm::Patience)
        .diff_slices(&old, &new);
    let mut out: Vec<TextChunk> = Vec::new();
    for change in diff.iter_all_changes() {
        let op = match change.tag() {
            ChangeTag::Equal => DiffOp::Equal,
            ChangeTag::Delete => DiffOp::Delete,
            ChangeTag::Insert => DiffOp::Insert,
        };
        match out.last_mut() {
            Some(last) if last.op == op => last.text.push_str(change.value()),
            _ => out.push(TextChunk {
                op,
                text: (*change.value()).to_string(),
            }),
        }
    }
    out
}

/// Every element of a document with its position: (list key, index).
fn positions(doc: &UseCaseDocument) -> Vec<(String, Side)> {
    let labels = labels(doc);
    let side = |list: &'static str, branch: Option<&str>, id: &str, text: &str| Side {
        list,
        branch_id: branch.map(str::to_string),
        label: labels.get(id).cloned(),
        text: text.to_string(),
        from_step: None,
        end: None,
        kind: None,
    };
    let mut out = Vec::new();
    for e in &doc.preconditions {
        out.push((e.id.clone(), side("preconditions", None, &e.id, &e.text)));
    }
    for e in &doc.main {
        out.push((e.id.clone(), side("main", None, &e.id, &e.text)));
    }
    for b in &doc.branches {
        let mut branch = side("branches", None, &b.id, &b.title);
        branch.from_step = Some(b.from_step.clone());
        branch.end = Some(b.end.clone());
        branch.kind = Some(b.kind);
        out.push((b.id.clone(), branch));
        for e in &b.steps {
            out.push((e.id.clone(), side("branch", Some(&b.id), &e.id, &e.text)));
        }
    }
    for e in &doc.postconditions {
        out.push((e.id.clone(), side("postconditions", None, &e.id, &e.text)));
    }
    out
}

/// The list an element sits in, as a comparable key.
fn list_key(side: &Side) -> String {
    match &side.branch_id {
        Some(b) => format!("branch:{b}"),
        None => side.list.to_string(),
    }
}

/// Ids of a longest common subsequence of `a` and `b`.
fn lcs(a: &[&str], b: &[&str]) -> HashSet<String> {
    let (n, m) = (a.len(), b.len());
    let mut t = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            t[i][j] = if a[i] == b[j] {
                t[i + 1][j + 1] + 1
            } else {
                t[i + 1][j].max(t[i][j + 1])
            };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, HashSet::new());
    while i < n && j < m {
        if a[i] == b[j] {
            out.insert(a[i].to_string());
            i += 1;
            j += 1;
        } else if t[i][j + 1] >= t[i + 1][j] {
            // On a tie, keep the old order's element: the one the new order
            // brought forward is the one that moved (a step dragged up).
            j += 1;
        } else {
            i += 1;
        }
    }
    out
}

/// Compare `old` with `new`.
pub fn diff_documents(
    old: &UseCaseDocument,
    new: &UseCaseDocument,
    include_unchanged: bool,
) -> DocumentDiff {
    let old_pos = positions(old);
    let new_pos = positions(new);
    let old_by_id: HashMap<&str, &Side> = old_pos.iter().map(|(id, s)| (id.as_str(), s)).collect();
    let new_by_id: HashMap<&str, &Side> = new_pos.iter().map(|(id, s)| (id.as_str(), s)).collect();

    // Elements that kept their list but not their order relative to the
    // others that did: per list, those outside a longest common subsequence.
    let mut stayed: HashSet<String> = HashSet::new();
    let lists: HashSet<String> = old_pos.iter().map(|(_, s)| list_key(s)).collect();
    for list in lists {
        let in_list = |pos: &[(String, Side)], other: &HashMap<&str, &Side>| -> Vec<String> {
            pos.iter()
                .filter(|(id, s)| {
                    list_key(s) == list
                        && s.list != "branches"
                        && other.get(id.as_str()).is_some_and(|o| list_key(o) == list)
                })
                .map(|(id, _)| id.clone())
                .collect()
        };
        let a = in_list(&old_pos, &new_by_id);
        let b = in_list(&new_pos, &old_by_id);
        let a: Vec<&str> = a.iter().map(String::as_str).collect();
        let b: Vec<&str> = b.iter().map(String::as_str).collect();
        stayed.extend(lcs(&a, &b));
    }

    let mut summary = Summary::default();
    let mut elements = Vec::new();
    let mut push = |id: &str,
                    kind: ElementKind,
                    changes: Vec<Change>,
                    old: Option<&Side>,
                    new: Option<&Side>,
                    text_diff: Option<Vec<TextChunk>>| {
        for c in &changes {
            match c {
                Change::Added => summary.added += 1,
                Change::Removed => summary.removed += 1,
                Change::Edited => summary.edited += 1,
                Change::Moved => summary.moved += 1,
                Change::Rebranched => summary.rebranched += 1,
            }
        }
        if changes.is_empty() && !include_unchanged {
            return;
        }
        elements.push(ElementDiff {
            id: id.to_string(),
            kind,
            changes,
            old: old.cloned(),
            new: new.cloned(),
            text_diff,
        });
    };

    for (id, new_side) in &new_pos {
        let kind = ElementKind::of(id).unwrap_or(ElementKind::Step);
        let Some(old_side) = old_by_id.get(id.as_str()) else {
            push(id, kind, vec![Change::Added], None, Some(new_side), None);
            continue;
        };
        let mut changes = Vec::new();
        let mut text_diff = None;
        if old_side.text != new_side.text || old_side.kind != new_side.kind {
            changes.push(Change::Edited);
            if old_side.text != new_side.text {
                text_diff = Some(word_diff(&old_side.text, &new_side.text));
            }
        }
        if kind == ElementKind::Branch {
            if old_side.from_step != new_side.from_step || old_side.end != new_side.end {
                changes.push(Change::Rebranched);
            }
        } else if list_key(old_side) != list_key(new_side) || !stayed.contains(id) {
            changes.push(Change::Moved);
        }
        push(id, kind, changes, Some(old_side), Some(new_side), text_diff);
    }
    for (id, old_side) in &old_pos {
        if !new_by_id.contains_key(id.as_str()) {
            let kind = ElementKind::of(id).unwrap_or(ElementKind::Step);
            push(id, kind, vec![Change::Removed], Some(old_side), None, None);
        }
    }

    let mut header = BTreeMap::new();
    let mut field = |name: &'static str, old: serde_json::Value, new: serde_json::Value| {
        if old != new {
            header.insert(name, FieldChange { old, new });
        }
    };
    field("title", old.title.clone().into(), new.title.clone().into());
    field(
        "primary_actor",
        old.primary_actor.clone().into(),
        new.primary_actor.clone().into(),
    );
    field(
        "secondary_actors",
        old.secondary_actors.clone().into(),
        new.secondary_actors.clone().into(),
    );
    field(
        "summary",
        old.summary.clone().into(),
        new.summary.clone().into(),
    );

    DocumentDiff {
        summary,
        header,
        elements,
    }
}

#[cfg(test)]
mod tests {
    use super::super::operations::{Operation, apply_operations};
    use super::super::tests::sample;
    use super::*;
    use serde_json::json;

    fn changed(ops: serde_json::Value) -> UseCaseDocument {
        let mut doc = sample();
        let ops: Vec<Operation> = serde_json::from_value(ops).unwrap();
        apply_operations(&mut doc, ops).unwrap();
        doc
    }

    fn by_id<'a>(diff: &'a DocumentDiff, id: &str) -> &'a ElementDiff {
        diff.elements
            .iter()
            .find(|e| e.id == id)
            .unwrap_or_else(|| panic!("{id} in {diff:#?}"))
    }

    #[test]
    fn identical_documents_have_no_changes() {
        let diff = diff_documents(&sample(), &sample(), false);
        assert_eq!(diff.summary, Summary::default());
        assert!(diff.elements.is_empty() && diff.header.is_empty());
        assert_eq!(
            diff_documents(&sample(), &sample(), true).elements.len(),
            10
        );
    }

    #[test]
    fn a_move_is_one_moved_step_not_a_removal() {
        let new = changed(json!([{ "op": "move_element", "id": "s_C3mz", "after": "s_A1x9" }]));
        let diff = diff_documents(&sample(), &new, false);
        assert_eq!(
            diff.summary,
            Summary {
                moved: 1,
                ..Summary::default()
            }
        );
        let moved = by_id(&diff, "s_C3mz");
        assert_eq!(moved.changes, vec![Change::Moved]);
        assert_eq!(moved.old.as_ref().unwrap().label.as_deref(), Some("3"));
        assert_eq!(moved.new.as_ref().unwrap().label.as_deref(), Some("2"));
    }

    #[test]
    fn edits_carry_a_word_diff() {
        let new = changed(
            json!([{ "op": "update_text", "id": "s_D8pw", "text": "System generates the file in the selected format and offers it for download." }]),
        );
        let diff = diff_documents(&sample(), &new, false);
        let edited = by_id(&diff, "s_D8pw");
        assert_eq!(edited.changes, vec![Change::Edited]);
        let chunks = edited.text_diff.as_ref().unwrap();
        assert!(
            chunks.contains(&TextChunk {
                op: DiffOp::Delete,
                text: "chosen".into()
            }),
            "{chunks:?}"
        );
        assert!(
            chunks.contains(&TextChunk {
                op: DiffOp::Insert,
                text: "selected".into()
            }),
            "{chunks:?}"
        );
        let rebuilt: String = chunks
            .iter()
            .filter(|c| c.op != DiffOp::Delete)
            .map(|c| c.text.as_str())
            .collect();
        assert_eq!(rebuilt, new.text_of("s_D8pw").unwrap());
    }

    #[test]
    fn added_removed_rebranched_and_header() {
        let new = changed(json!([
            { "op": "add_element", "list": "postconditions", "element": { "id": "q_New1", "text": "Logged." } },
            { "op": "remove_branch", "id": "b_F4il" },
            { "op": "update_branch", "id": "b_N0s1", "from_step": "s_Bf20" },
            { "op": "set_header", "title": "Export a selection" }
        ]));
        let diff = diff_documents(&sample(), &new, false);
        assert_eq!(by_id(&diff, "q_New1").changes, vec![Change::Added]);
        assert_eq!(by_id(&diff, "b_F4il").changes, vec![Change::Removed]);
        assert_eq!(by_id(&diff, "s_R7kk").changes, vec![Change::Removed]);
        let rebranched = by_id(&diff, "b_N0s1");
        assert_eq!(rebranched.changes, vec![Change::Rebranched]);
        assert_eq!(
            rebranched.new.as_ref().unwrap().label.as_deref(),
            Some("A2")
        );
        assert_eq!(diff.header["title"].new, json!("Export a selection"));
        assert_eq!(
            diff.summary,
            Summary {
                added: 1,
                removed: 2,
                rebranched: 1,
                ..Summary::default()
            }
        );
        // Removed elements come after the new document's.
        assert_eq!(diff.elements.last().unwrap().changes, vec![Change::Removed]);
    }

    #[test]
    fn a_step_moved_into_a_branch_is_moved() {
        let new = changed(
            json!([{ "op": "move_element", "id": "s_A1x9", "list": { "branch": "b_F4il" } }]),
        );
        let diff = diff_documents(&sample(), &new, false);
        let moved = by_id(&diff, "s_A1x9");
        assert_eq!(moved.changes, vec![Change::Moved]);
        assert_eq!(
            moved.new.as_ref().unwrap().branch_id.as_deref(),
            Some("b_F4il")
        );
        // The remaining main steps did not move relative to each other.
        assert!(
            diff.elements
                .iter()
                .all(|e| e.id == "s_A1x9" || e.kind == ElementKind::Branch || e.changes.is_empty()),
            "{diff:#?}"
        );
    }
}
