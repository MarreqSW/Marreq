// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Dependency Structure Matrix (issue #328): a requirement × requirement matrix
//! built from `requirement_version_links`. A cell in row *i*, column *j* means
//! requirement *i*'s current version links to requirement *j*.
//!
//! [`build_dsm`] is pure (no repository access) so ordering, loop detection and
//! the "upstream changed" marker are unit-testable with plain structs;
//! [`load_dsm`] gathers the inputs from a repository.

use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};

use chrono::NaiveDateTime;
use rocket::serde::Serialize;

use crate::repository::errors::RepoError;
use crate::repository::{
    LookupRepository, RequirementVersionLinksRepository, RequirementsRepository,
};
use crate::services::requirement_service::REQUIREMENT_VERSION_LINK_TYPES;

/// Link types drawn when the caller does not choose: every type except
/// `RELATES_TO`, which is informational and would clutter the matrix.
pub fn default_link_types() -> Vec<String> {
    REQUIREMENT_VERSION_LINK_TYPES
        .iter()
        .filter(|t| **t != "RELATES_TO")
        .map(|t| (*t).to_string())
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DsmOrder {
    /// Category blocks along the diagonal, requirement hierarchy inside each.
    #[default]
    Hierarchy,
    /// Dependencies before dependants (loops kept together), so acyclic
    /// dependencies fall below the diagonal and feedback shows above it.
    Partition,
}

impl DsmOrder {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "hierarchy" => Some(Self::Hierarchy),
            "partition" => Some(Self::Partition),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Hierarchy => "hierarchy",
            Self::Partition => "partition",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DsmOptions {
    pub link_types: Vec<String>,
    pub category_id: Option<i32>,
    pub root_id: Option<i32>,
    pub order: DsmOrder,
}

/// One requirement as seen by the builder (current version fields).
#[derive(Debug, Clone)]
pub struct DsmRequirementInput {
    pub id: i32,
    pub current_version_id: Option<i32>,
    pub reference_code: String,
    pub title: String,
    pub category_id: i32,
    pub approval_state: String,
    pub approved_at: Option<NaiveDateTime>,
    /// Creation time of the current version.
    pub current_version_created_at: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct DsmLinkInput {
    pub id: i32,
    pub source_version_id: i32,
    pub target_version_id: i32,
    pub link_type: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Default)]
pub struct DsmInput {
    pub requirements: Vec<DsmRequirementInput>,
    pub links: Vec<DsmLinkInput>,
    /// Category id → title.
    pub categories: HashMap<i32, String>,
    /// Version id → requirement id for versions that are not current.
    pub extra_versions: HashMap<i32, i32>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct DsmRequirement {
    pub id: i32,
    pub index: usize,
    pub reference_code: String,
    pub title: String,
    pub approval_state: String,
    pub category_id: i32,
    pub category: String,
    pub parent_id: Option<i32>,
    pub depth: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct DsmCell {
    pub row: usize,
    pub col: usize,
    pub link_types: Vec<String>,
    pub link_ids: Vec<i32>,
    /// Source is approved and the target's current version is newer than that approval.
    pub upstream_changed: bool,
    /// Source and target belong to the same dependency loop.
    pub in_loop: bool,
}

/// Contiguous block of rows/columns (inclusive indices).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct DsmGroup {
    pub label: String,
    pub start: usize,
    pub end: usize,
}

/// A dependency loop: all requirements of one strongly connected component and
/// one concrete cycle through them (`path[0] → path[1] → … → path[0]`).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct DsmLoop {
    pub requirement_ids: Vec<i32>,
    pub path: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct DsmStats {
    pub requirements: usize,
    pub links: usize,
    pub cells: usize,
    pub loops: usize,
    pub upstream_changed: usize,
    /// Links between a requirement in scope and one outside it (not drawn).
    pub external_links: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct Dsm {
    pub order: String,
    pub link_types: Vec<String>,
    pub requirements: Vec<DsmRequirement>,
    pub cells: Vec<DsmCell>,
    pub groups: Vec<DsmGroup>,
    pub loops: Vec<DsmLoop>,
    pub stats: DsmStats,
}

/// Build the matrix. Unknown link ids/versions are ignored.
pub fn build_dsm(input: &DsmInput, options: &DsmOptions) -> Dsm {
    let reqs: HashMap<i32, &DsmRequirementInput> =
        input.requirements.iter().map(|r| (r.id, r)).collect();

    let mut version_to_req: HashMap<i32, i32> = input.extra_versions.clone();
    for r in &input.requirements {
        if let Some(v) = r.current_version_id {
            version_to_req.insert(v, r.id);
        }
    }
    let req_of = |version_id: i32| version_to_req.get(&version_id).copied();

    // Parent = target of the requirement's earliest outgoing link (any type),
    // matching how `parent_id` is derived elsewhere.
    let mut sorted_links: Vec<&DsmLinkInput> = input.links.iter().collect();
    sorted_links.sort_by_key(|l| (l.created_at, l.id));
    let mut parent: HashMap<i32, i32> = HashMap::new();
    for link in &sorted_links {
        let (Some(s), Some(t)) = (
            req_of(link.source_version_id),
            req_of(link.target_version_id),
        ) else {
            continue;
        };
        if s != t && reqs.contains_key(&s) && reqs.contains_key(&t) {
            parent.entry(s).or_insert(t);
        }
    }

    // Scope: category and/or subtree of `root_id` (via the parent relation).
    let mut in_scope: HashSet<i32> = reqs
        .keys()
        .copied()
        .filter(|id| {
            options
                .category_id
                .is_none_or(|c| reqs[id].category_id == c)
        })
        .collect();
    if let Some(root) = options.root_id {
        let mut children: HashMap<i32, Vec<i32>> = HashMap::new();
        for (child, p) in &parent {
            children.entry(*p).or_default().push(*child);
        }
        let mut subtree = HashSet::new();
        let mut stack = vec![root];
        while let Some(id) = stack.pop() {
            if reqs.contains_key(&id) && subtree.insert(id) {
                stack.extend(children.get(&id).into_iter().flatten().copied());
            }
        }
        in_scope.retain(|id| subtree.contains(id));
    }

    // Edges between requirements (source depends on target), deduplicated.
    let selected: HashSet<&str> = options.link_types.iter().map(String::as_str).collect();
    let mut edges: BTreeMap<(i32, i32), (BTreeSet<String>, Vec<i32>)> = BTreeMap::new();
    let mut drawn_links = 0usize;
    let mut external_links = 0usize;
    for link in &sorted_links {
        if !selected.contains(link.link_type.as_str()) {
            continue;
        }
        let (Some(s), Some(t)) = (
            req_of(link.source_version_id),
            req_of(link.target_version_id),
        ) else {
            continue;
        };
        if s == t {
            continue;
        }
        match (in_scope.contains(&s), in_scope.contains(&t)) {
            (true, true) => {
                let entry = edges.entry((s, t)).or_default();
                entry.0.insert(link.link_type.clone());
                entry.1.push(link.id);
                drawn_links += 1;
            }
            (false, false) => {}
            _ => external_links += 1,
        }
    }

    // Hierarchy order: category blocks, depth-first by parent inside each.
    let category_label = |id: i32| input.categories.get(&id).cloned();
    let mut by_category: BTreeMap<(u8, String, i32), Vec<i32>> = BTreeMap::new();
    for id in &in_scope {
        let r = reqs[id];
        let key = match category_label(r.category_id) {
            Some(label) => (0, label.to_lowercase(), r.category_id),
            None => (1, String::new(), r.category_id),
        };
        by_category.entry(key).or_default().push(*id);
    }
    let mut hierarchy: Vec<i32> = Vec::with_capacity(in_scope.len());
    let mut depth: HashMap<i32, usize> = HashMap::new();
    let mut groups = Vec::new();
    for ((_, _, category_id), members) in &by_category {
        let start = hierarchy.len();
        let member_set: HashSet<i32> = members.iter().copied().collect();
        let mut children: HashMap<i32, Vec<i32>> = HashMap::new();
        let mut roots = Vec::new();
        for id in members {
            match parent.get(id) {
                Some(p) if member_set.contains(p) => children.entry(*p).or_default().push(*id),
                _ => roots.push(*id),
            }
        }
        let by_code =
            |a: &i32, b: &i32| natural_cmp(&reqs[a].reference_code, &reqs[b].reference_code);
        roots.sort_by(by_code);
        for kids in children.values_mut() {
            kids.sort_by(by_code);
        }
        let mut visited = HashSet::new();
        let mut walk = |root: i32, hierarchy: &mut Vec<i32>, depth: &mut HashMap<i32, usize>| {
            let mut stack = vec![(root, 0usize)];
            while let Some((id, d)) = stack.pop() {
                if !visited.insert(id) {
                    continue;
                }
                hierarchy.push(id);
                depth.insert(id, d);
                for kid in children.get(&id).into_iter().flatten().rev() {
                    stack.push((*kid, d + 1));
                }
            }
        };
        for root in roots {
            walk(root, &mut hierarchy, &mut depth);
        }
        // Members only reachable through a parent cycle: append by code.
        let mut rest: Vec<i32> = members
            .iter()
            .copied()
            .filter(|id| !depth.contains_key(id))
            .collect();
        rest.sort_by(by_code);
        for id in rest {
            walk(id, &mut hierarchy, &mut depth);
        }
        let label = category_label(*category_id).unwrap_or_else(|| "Uncategorised".into());
        groups.push(DsmGroup {
            label,
            start,
            end: hierarchy.len() - 1,
        });
    }
    let hierarchy_index: HashMap<i32, usize> = hierarchy
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i))
        .collect();

    // Loops: strongly connected components over the drawn edges.
    let n = hierarchy.len();
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (s, t) in edges.keys() {
        adjacency[hierarchy_index[s]].push(hierarchy_index[t]);
    }
    let component = tarjan_scc(&adjacency);
    let mut members_of: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (node, comp) in component.iter().enumerate() {
        members_of.entry(*comp).or_default().push(node);
    }
    let loop_components: HashSet<usize> = members_of
        .iter()
        .filter(|(_, m)| m.len() > 1)
        .map(|(c, _)| *c)
        .collect();

    let order: Vec<i32> = match options.order {
        DsmOrder::Hierarchy => hierarchy.clone(),
        DsmOrder::Partition => partition_order(&adjacency, &component, &members_of)
            .into_iter()
            .map(|node| hierarchy[node])
            .collect(),
    };
    let index: HashMap<i32, usize> = order.iter().enumerate().map(|(i, id)| (*id, i)).collect();

    let mut loops: Vec<DsmLoop> = loop_components
        .iter()
        .map(|comp| {
            let nodes = &members_of[comp];
            let mut ids: Vec<i32> = nodes.iter().map(|n| hierarchy[*n]).collect();
            ids.sort_by_key(|id| index[id]);
            let path = cycle_path(&adjacency, &component, *comp, nodes)
                .into_iter()
                .map(|n| hierarchy[n])
                .collect();
            DsmLoop {
                requirement_ids: ids,
                path,
            }
        })
        .collect();
    loops.sort_by_key(|l| index[&l.requirement_ids[0]]);

    let mut cells: Vec<DsmCell> = edges
        .into_iter()
        .map(|((s, t), (types, link_ids))| {
            let source = reqs[&s];
            let target = reqs[&t];
            let upstream_changed = source.approval_state == "approved"
                && source
                    .approved_at
                    .is_some_and(|approved| target.current_version_created_at > approved);
            let (cs, ct) = (
                component[hierarchy_index[&s]],
                component[hierarchy_index[&t]],
            );
            DsmCell {
                row: index[&s],
                col: index[&t],
                link_types: types.into_iter().collect(),
                link_ids,
                upstream_changed,
                in_loop: cs == ct && loop_components.contains(&cs),
            }
        })
        .collect();
    cells.sort_by_key(|c| (c.row, c.col));

    let requirements = order
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let r = reqs[id];
            DsmRequirement {
                id: r.id,
                index: i,
                reference_code: r.reference_code.clone(),
                title: r.title.clone(),
                approval_state: r.approval_state.clone(),
                category_id: r.category_id,
                category: category_label(r.category_id).unwrap_or_else(|| "Uncategorised".into()),
                parent_id: parent.get(id).copied(),
                depth: depth.get(id).copied().unwrap_or(0),
            }
        })
        .collect::<Vec<_>>();

    let stats = DsmStats {
        requirements: requirements.len(),
        links: drawn_links,
        cells: cells.len(),
        loops: loops.len(),
        upstream_changed: cells.iter().filter(|c| c.upstream_changed).count(),
        external_links,
    };
    Dsm {
        order: options.order.as_str().to_string(),
        link_types: options.link_types.clone(),
        requirements,
        cells,
        // Category blocks only make sense in hierarchy order.
        groups: if options.order == DsmOrder::Hierarchy {
            groups
        } else {
            Vec::new()
        },
        loops,
        stats,
    }
}

/// Load the inputs for `project_id` from the repository and build the matrix.
pub fn load_dsm<R>(repo: &R, project_id: i32, options: &DsmOptions) -> Result<Dsm, RepoError>
where
    R: RequirementsRepository + RequirementVersionLinksRepository + LookupRepository + ?Sized,
{
    let requirements: Vec<DsmRequirementInput> = repo
        .get_requirements_by_project(project_id)?
        .into_iter()
        .map(|r| DsmRequirementInput {
            id: r.id,
            current_version_id: r.current_version_id,
            reference_code: r.reference_code,
            title: r.title,
            category_id: r.category_id,
            approval_state: r.approval_state,
            approved_at: r.approved_at,
            current_version_created_at: r.update_date,
        })
        .collect();
    if let Some(root) = options.root_id
        && !requirements.iter().any(|r| r.id == root)
    {
        return Err(RepoError::NotFound);
    }
    let links: Vec<DsmLinkInput> = repo
        .list_links_by_project(project_id, None, None, None)?
        .into_iter()
        .map(|l| DsmLinkInput {
            id: l.id,
            source_version_id: l.source_version_id,
            target_version_id: l.target_version_id,
            link_type: l.link_type,
            created_at: l.created_at,
        })
        .collect();

    // Links follow edits to the current version, so only links pinned to an
    // older version need a lookup.
    let current: HashSet<i32> = requirements
        .iter()
        .filter_map(|r| r.current_version_id)
        .collect();
    let mut pinned: Vec<i32> = links
        .iter()
        .flat_map(|l| [l.source_version_id, l.target_version_id])
        .filter(|v| !current.contains(v))
        .collect();
    pinned.sort_unstable();
    pinned.dedup();
    let extra_versions = if pinned.is_empty() {
        HashMap::new()
    } else {
        repo.requirement_ids_for_versions(&pinned)?
    };

    let categories = repo
        .get_categories_by_project(project_id)?
        .into_iter()
        .map(|c| (c.id, c.title))
        .collect();

    Ok(build_dsm(
        &DsmInput {
            requirements,
            links,
            categories,
            extra_versions,
        },
        options,
    ))
}

/// Natural ordering for reference codes: `REQ-2` before `REQ-10`.
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let (mut ai, mut bi) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (ai.peek(), bi.peek()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let take = |it: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut s = String::new();
                    while let Some(c) = it.peek().copied().filter(char::is_ascii_digit) {
                        s.push(c);
                        it.next();
                    }
                    s
                };
                let (na, nb) = (take(&mut ai), take(&mut bi));
                let (ta, tb) = (na.trim_start_matches('0'), nb.trim_start_matches('0'));
                let ord = ta.len().cmp(&tb.len()).then_with(|| ta.cmp(tb));
                if ord != Ordering::Equal {
                    return ord;
                }
            }
            (Some(x), Some(y)) => {
                let ord = x.to_ascii_lowercase().cmp(&y.to_ascii_lowercase());
                if ord != Ordering::Equal {
                    return ord;
                }
                ai.next();
                bi.next();
            }
        }
    }
}

/// Iterative Tarjan: component id per node. Ids are assigned in completion
/// order (a component finishes after every component it can reach).
fn tarjan_scc(adjacency: &[Vec<usize>]) -> Vec<usize> {
    let n = adjacency.len();
    let mut index = vec![usize::MAX; n];
    let mut low = vec![0; n];
    let mut on_stack = vec![false; n];
    let mut stack = Vec::new();
    let mut component = vec![usize::MAX; n];
    let (mut next_index, mut next_component) = (0, 0);
    for start in 0..n {
        if index[start] != usize::MAX {
            continue;
        }
        let mut work = vec![(start, 0usize)];
        while let Some((node, edge)) = work.pop() {
            if edge == 0 {
                index[node] = next_index;
                low[node] = next_index;
                next_index += 1;
                stack.push(node);
                on_stack[node] = true;
            }
            if let Some(&next) = adjacency[node].get(edge) {
                work.push((node, edge + 1));
                if index[next] == usize::MAX {
                    work.push((next, 0));
                } else if on_stack[next] {
                    low[node] = low[node].min(index[next]);
                }
                continue;
            }
            if low[node] == index[node] {
                while let Some(member) = stack.pop() {
                    on_stack[member] = false;
                    component[member] = next_component;
                    if member == node {
                        break;
                    }
                }
                next_component += 1;
            }
            if let Some(&(caller, _)) = work.last() {
                low[caller] = low[caller].min(low[node]);
            }
        }
    }
    component
}

/// Dependencies first: for an edge `s → t` (s depends on t), t's component is
/// placed before s's. Ties go to the lowest hierarchy position; members of a
/// component stay together in hierarchy order.
fn partition_order(
    adjacency: &[Vec<usize>],
    component: &[usize],
    members_of: &BTreeMap<usize, Vec<usize>>,
) -> Vec<usize> {
    let mut blocked_by: HashMap<usize, usize> = members_of.keys().map(|c| (*c, 0)).collect();
    let mut unblocks: HashMap<usize, BTreeSet<usize>> = HashMap::new();
    for (s, targets) in adjacency.iter().enumerate() {
        for t in targets {
            let (cs, ct) = (component[s], component[*t]);
            if cs != ct && unblocks.entry(ct).or_default().insert(cs) {
                *blocked_by.get_mut(&cs).unwrap() += 1;
            }
        }
    }
    let first = |c: &usize| members_of[c][0];
    let mut ready: BinaryHeap<Reverse<(usize, usize)>> = blocked_by
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(c, _)| Reverse((first(c), *c)))
        .collect();
    let mut out = Vec::with_capacity(component.len());
    while let Some(Reverse((_, comp))) = ready.pop() {
        out.extend(members_of[&comp].iter().copied());
        for next in unblocks.get(&comp).into_iter().flatten() {
            let count = blocked_by.get_mut(next).unwrap();
            *count -= 1;
            if *count == 0 {
                ready.push(Reverse((first(next), *next)));
            }
        }
    }
    out
}

/// A shortest cycle through the component's first member (BFS inside the component).
fn cycle_path(
    adjacency: &[Vec<usize>],
    component: &[usize],
    comp: usize,
    nodes: &[usize],
) -> Vec<usize> {
    let start = nodes[0];
    let mut previous: HashMap<usize, usize> = HashMap::new();
    let mut queue = VecDeque::from([start]);
    while let Some(node) = queue.pop_front() {
        for &next in &adjacency[node] {
            if component[next] != comp {
                continue;
            }
            if next == start {
                let mut path = vec![node];
                let mut cur = node;
                while cur != start {
                    cur = previous[&cur];
                    path.push(cur);
                }
                path.reverse();
                return path;
            }
            if let std::collections::hash_map::Entry::Vacant(e) = previous.entry(next) {
                e.insert(node);
                queue.push_back(next);
            }
        }
    }
    nodes.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(day: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 1, day)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    /// Requirement `id` with current version `id * 10`.
    fn req(id: i32, code: &str, category: i32) -> DsmRequirementInput {
        DsmRequirementInput {
            id,
            current_version_id: Some(id * 10),
            reference_code: code.into(),
            title: format!("Title {id}"),
            category_id: category,
            approval_state: "draft".into(),
            approved_at: None,
            current_version_created_at: at(1),
        }
    }

    /// Link from requirement `s` to requirement `t` (current versions).
    fn link(id: i32, s: i32, t: i32, link_type: &str) -> DsmLinkInput {
        DsmLinkInput {
            id,
            source_version_id: s * 10,
            target_version_id: t * 10,
            link_type: link_type.into(),
            created_at: at(1) + chrono::Duration::seconds(id as i64),
        }
    }

    fn input(reqs: Vec<DsmRequirementInput>, links: Vec<DsmLinkInput>) -> DsmInput {
        DsmInput {
            requirements: reqs,
            links,
            categories: HashMap::from([(1, "Power".into()), (2, "Comms".into())]),
            extra_versions: HashMap::new(),
        }
    }

    fn opts() -> DsmOptions {
        DsmOptions {
            link_types: default_link_types(),
            ..Default::default()
        }
    }

    fn codes(dsm: &Dsm) -> Vec<&str> {
        dsm.requirements
            .iter()
            .map(|r| r.reference_code.as_str())
            .collect()
    }

    fn cell<'a>(dsm: &'a Dsm, from: &str, to: &str) -> Option<&'a DsmCell> {
        let idx = |code: &str| {
            dsm.requirements
                .iter()
                .position(|r| r.reference_code == code)
                .unwrap()
        };
        let (r, c) = (idx(from), idx(to));
        dsm.cells.iter().find(|cell| cell.row == r && cell.col == c)
    }

    #[test]
    fn one_cell_per_linked_pair_in_row_depends_on_column() {
        let data = input(
            vec![req(1, "PWR-1", 1), req(2, "PWR-2", 1), req(3, "PWR-3", 1)],
            vec![link(1, 2, 1, "DERIVES_FROM"), link(2, 3, 1, "DEPENDS_ON")],
        );
        let dsm = build_dsm(&data, &opts());
        assert_eq!(dsm.cells.len(), 2);
        assert_eq!(
            cell(&dsm, "PWR-2", "PWR-1").unwrap().link_types,
            ["DERIVES_FROM"]
        );
        assert!(cell(&dsm, "PWR-1", "PWR-2").is_none());
        assert_eq!(dsm.stats.links, 2);
    }

    #[test]
    fn multiple_links_between_a_pair_share_one_cell() {
        let data = input(
            vec![req(1, "A", 1), req(2, "B", 1)],
            vec![link(1, 2, 1, "REFINES"), link(2, 2, 1, "DEPENDS_ON")],
        );
        let dsm = build_dsm(&data, &opts());
        assert_eq!(dsm.cells.len(), 1);
        assert_eq!(dsm.cells[0].link_types, ["DEPENDS_ON", "REFINES"]);
        assert_eq!(dsm.cells[0].link_ids, [1, 2]);
    }

    #[test]
    fn link_type_filter_and_relates_to_default_off() {
        let data = input(
            vec![req(1, "A", 1), req(2, "B", 1)],
            vec![link(1, 2, 1, "RELATES_TO")],
        );
        assert!(build_dsm(&data, &opts()).cells.is_empty());
        let all = DsmOptions {
            link_types: vec!["RELATES_TO".into()],
            ..Default::default()
        };
        assert_eq!(build_dsm(&data, &all).cells.len(), 1);
    }

    #[test]
    fn hierarchy_order_groups_categories_and_nests_children() {
        let data = input(
            vec![
                req(1, "PWR-10", 1),
                req(2, "PWR-2", 1),
                req(3, "PWR-2.1", 1),
                req(4, "COM-1", 2),
                req(5, "MISC-1", 99),
            ],
            // PWR-2.1 is a child of PWR-10 (its first link); later links don't change the parent.
            vec![link(1, 3, 1, "DERIVES_FROM"), link(2, 3, 2, "DEPENDS_ON")],
        );
        let dsm = build_dsm(&data, &opts());
        // Categories by name (Comms, Power), unknown last; natural code order inside.
        assert_eq!(
            codes(&dsm),
            ["COM-1", "PWR-2", "PWR-10", "PWR-2.1", "MISC-1"]
        );
        let group_labels: Vec<_> = dsm
            .groups
            .iter()
            .map(|g| (g.label.as_str(), g.start, g.end))
            .collect();
        assert_eq!(
            group_labels,
            [("Comms", 0, 0), ("Power", 1, 3), ("Uncategorised", 4, 4)]
        );
        let child = dsm
            .requirements
            .iter()
            .find(|r| r.reference_code == "PWR-2.1")
            .unwrap();
        assert_eq!((child.parent_id, child.depth), (Some(1), 1));
    }

    #[test]
    fn loop_across_link_types_is_one_component_with_a_path() {
        let data = input(
            vec![
                req(1, "A", 1),
                req(2, "B", 1),
                req(3, "C", 1),
                req(4, "D", 1),
            ],
            vec![
                link(1, 1, 2, "DEPENDS_ON"),
                link(2, 2, 3, "REFINES"),
                link(3, 3, 1, "DERIVES_FROM"),
                link(4, 4, 1, "DEPENDS_ON"),
            ],
        );
        let dsm = build_dsm(&data, &opts());
        assert_eq!(dsm.loops.len(), 1);
        let l = &dsm.loops[0];
        assert_eq!(l.requirement_ids.len(), 3);
        assert_eq!(l.path.len(), 3);
        // Every step of the path is a drawn cell, and it closes back on the start.
        for (i, from) in l.path.iter().enumerate() {
            let to = l.path[(i + 1) % l.path.len()];
            let row = dsm.requirements.iter().position(|r| r.id == *from).unwrap();
            let col = dsm.requirements.iter().position(|r| r.id == to).unwrap();
            assert!(
                dsm.cells
                    .iter()
                    .any(|c| c.row == row && c.col == col && c.in_loop)
            );
        }
        assert!(!cell(&dsm, "D", "A").unwrap().in_loop);
    }

    #[test]
    fn partition_order_puts_dependencies_first_and_keeps_loops_together() {
        // D depends on C, C depends on B, B depends on A (chain, reversed codes)
        // plus a loop X <-> Y that depends on A.
        let data = input(
            vec![
                req(1, "D", 1),
                req(2, "C", 1),
                req(3, "B", 1),
                req(4, "A", 1),
                req(5, "X", 1),
                req(6, "Y", 1),
            ],
            vec![
                link(1, 1, 2, "DEPENDS_ON"),
                link(2, 2, 3, "DEPENDS_ON"),
                link(3, 3, 4, "DEPENDS_ON"),
                link(4, 5, 6, "DEPENDS_ON"),
                link(5, 6, 5, "DEPENDS_ON"),
                link(6, 5, 4, "DEPENDS_ON"),
            ],
        );
        let options = DsmOptions {
            order: DsmOrder::Partition,
            ..opts()
        };
        let dsm = build_dsm(&data, &options);
        // Acyclic dependencies all fall below the diagonal (col < row).
        for c in dsm.cells.iter().filter(|c| !c.in_loop) {
            assert!(c.col < c.row, "{c:?}");
        }
        let pos = |code: &str| {
            dsm.requirements
                .iter()
                .position(|r| r.reference_code == code)
                .unwrap()
        };
        assert!(pos("A") < pos("B") && pos("B") < pos("C") && pos("C") < pos("D"));
        assert_eq!(pos("X").abs_diff(pos("Y")), 1);
        assert!(dsm.groups.is_empty());
    }

    #[test]
    fn upstream_changed_when_target_version_is_newer_than_approval() {
        let mut source = req(1, "A", 1);
        source.approval_state = "approved".into();
        source.approved_at = Some(at(5));
        let mut newer = req(2, "B", 1);
        newer.current_version_created_at = at(9);
        let older = req(3, "C", 1);
        let mut draft_source = req(4, "D", 1);
        draft_source.approved_at = Some(at(5));
        let data = input(
            vec![source, newer.clone(), older, draft_source],
            vec![
                link(1, 1, 2, "DEPENDS_ON"),
                link(2, 1, 3, "DEPENDS_ON"),
                link(3, 4, 2, "DEPENDS_ON"),
            ],
        );
        let dsm = build_dsm(&data, &opts());
        assert!(cell(&dsm, "A", "B").unwrap().upstream_changed);
        assert!(!cell(&dsm, "A", "C").unwrap().upstream_changed);
        // Only approved sources are flagged.
        assert!(!cell(&dsm, "D", "B").unwrap().upstream_changed);
        assert_eq!(dsm.stats.upstream_changed, 1);
    }

    #[test]
    fn links_pinned_to_old_versions_are_mapped() {
        let mut data = input(
            vec![req(1, "A", 1), req(2, "B", 1)],
            vec![DsmLinkInput {
                id: 1,
                source_version_id: 20,
                target_version_id: 7, // old version of requirement 1
                link_type: "REFINES".into(),
                created_at: at(1),
            }],
        );
        assert!(build_dsm(&data, &opts()).cells.is_empty());
        data.extra_versions.insert(7, 1);
        assert!(cell(&build_dsm(&data, &opts()), "B", "A").is_some());
    }

    #[test]
    fn scope_by_category_and_root_counts_external_links() {
        let data = input(
            vec![
                req(1, "P-1", 1),
                req(2, "P-2", 1),
                req(3, "P-3", 1),
                req(4, "C-1", 2),
            ],
            vec![
                link(1, 2, 1, "DERIVES_FROM"),
                link(2, 3, 2, "DERIVES_FROM"),
                link(3, 3, 4, "DEPENDS_ON"),
            ],
        );
        let power = build_dsm(
            &data,
            &DsmOptions {
                category_id: Some(1),
                ..opts()
            },
        );
        assert_eq!(codes(&power), ["P-1", "P-2", "P-3"]);
        assert_eq!((power.stats.links, power.stats.external_links), (2, 1));

        let subtree = build_dsm(
            &data,
            &DsmOptions {
                root_id: Some(2),
                ..opts()
            },
        );
        assert_eq!(codes(&subtree), ["P-2", "P-3"]);
        assert_eq!((subtree.stats.links, subtree.stats.external_links), (1, 2));
    }

    #[test]
    fn natural_ordering_of_codes() {
        let mut v = vec!["REQ-10", "REQ-2", "REQ-2.1", "req-1", "REQ-002"];
        v.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(v, ["req-1", "REQ-002", "REQ-2", "REQ-2.1", "REQ-10"]);
    }

    #[test]
    fn large_project_builds() {
        let reqs: Vec<_> = (1..=1000)
            .map(|i| req(i, &format!("R-{i}"), i % 5))
            .collect();
        let links: Vec<_> = (1..=5000)
            .map(|i| {
                let s = (i % 1000) + 1;
                let t = ((i * 7) % 1000) + 1;
                link(i, s, if t == s { (t % 1000) + 1 } else { t }, "DEPENDS_ON")
            })
            .collect();
        let dsm = build_dsm(
            &input(reqs, links),
            &DsmOptions {
                order: DsmOrder::Partition,
                ..opts()
            },
        );
        assert_eq!(dsm.requirements.len(), 1000);
        assert!(dsm.stats.cells > 0 && dsm.stats.cells <= 5000);
    }
}
