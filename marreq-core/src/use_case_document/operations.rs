// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Draft operations: the small, validated edits the editor sends instead of
//! a whole document. A batch applies in order and atomically; the caller
//! validates the result with [`super::validate`].

use serde::{Deserialize, Deserializer, Serialize};

use super::{Branch, BranchEnd, BranchKind, Element, ElementKind, UseCaseDocument, labels, new_id};

/// `Some(None)` when the field is `null`, `None` when it is missing.
fn double_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListName {
    Preconditions,
    Main,
    Postconditions,
}

/// A list of elements: one of the three top-level lists, or a branch's steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ListRef {
    Named(ListName),
    Branch { branch: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewElement {
    #[serde(default)]
    pub id: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewBranch {
    #[serde(default)]
    pub id: Option<String>,
    pub kind: BranchKind,
    pub title: String,
    pub from_step: String,
    pub end: BranchEnd,
    #[serde(default)]
    pub steps: Vec<NewElement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cascade {
    Cascade,
}

/// What to do with the branches that depend on a removed step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Dependents {
    /// Remove them too.
    Cascade(Cascade),
    /// Make them start from, or rejoin at, another main step.
    Rebranch { rebranch_to: String },
}

/// One edit. `after` names the element to insert after: `null` means first
/// in the list, and leaving it out means last.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    SetHeader {
        #[serde(default)]
        title: Option<String>,
        #[serde(default, deserialize_with = "double_option")]
        primary_actor: Option<Option<String>>,
        #[serde(default)]
        secondary_actors: Option<Vec<String>>,
        #[serde(default, deserialize_with = "double_option")]
        summary: Option<Option<String>>,
    },
    AddElement {
        list: ListRef,
        #[serde(default, deserialize_with = "double_option")]
        after: Option<Option<String>>,
        element: NewElement,
    },
    UpdateText {
        id: String,
        text: String,
    },
    MoveElement {
        id: String,
        #[serde(default)]
        list: Option<ListRef>,
        #[serde(default, deserialize_with = "double_option")]
        after: Option<Option<String>>,
    },
    RemoveElement {
        id: String,
        #[serde(default)]
        dependents: Option<Dependents>,
    },
    AddBranch {
        branch: NewBranch,
    },
    UpdateBranch {
        id: String,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        kind: Option<BranchKind>,
        #[serde(default)]
        from_step: Option<String>,
        #[serde(default)]
        end: Option<BranchEnd>,
    },
    RemoveBranch {
        id: String,
    },
}

/// Something the server did on the client's behalf, for the editor's toasts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Effect {
    /// A branch's label changed because its origin moved.
    Relabelled {
        id: String,
        from: String,
        to: String,
    },
    /// A branch was removed with the step it depended on.
    Removed { id: String },
    /// A branch now starts from, or rejoins at, another step.
    Rebranched { id: String, from_step: String },
    /// The server generated an id for a new element.
    Created { id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationError {
    /// No element with this id.
    NotFound { index: usize, id: String },
    /// The step is the origin or rejoin target of these branches; the
    /// request must say what to do with them.
    HasDependents {
        index: usize,
        id: String,
        branches: Vec<String>,
    },
    /// Anything else that makes the operation impossible.
    Invalid { index: usize, message: String },
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationError::NotFound { index, id } => {
                write!(f, "operation {index}: no element \"{id}\"")
            }
            OperationError::HasDependents {
                index,
                id,
                branches,
            } => write!(
                f,
                "operation {index}: step \"{id}\" has dependent branches ({})",
                branches.join(", ")
            ),
            OperationError::Invalid { index, message } => {
                write!(f, "operation {index}: {message}")
            }
        }
    }
}

/// Where an element lives.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Location {
    Named(ListName),
    Branch(usize),
}

fn list_mut<'a>(doc: &'a mut UseCaseDocument, at: &Location) -> &'a mut Vec<Element> {
    match at {
        Location::Named(ListName::Preconditions) => &mut doc.preconditions,
        Location::Named(ListName::Main) => &mut doc.main,
        Location::Named(ListName::Postconditions) => &mut doc.postconditions,
        Location::Branch(i) => &mut doc.branches[*i].steps,
    }
}

fn find(doc: &UseCaseDocument, id: &str) -> Option<(Location, usize)> {
    let named = [
        (ListName::Preconditions, &doc.preconditions),
        (ListName::Main, &doc.main),
        (ListName::Postconditions, &doc.postconditions),
    ];
    for (name, list) in named {
        if let Some(i) = list.iter().position(|e| e.id == id) {
            return Some((Location::Named(name), i));
        }
    }
    for (b, branch) in doc.branches.iter().enumerate() {
        if let Some(i) = branch.steps.iter().position(|e| e.id == id) {
            return Some((Location::Branch(b), i));
        }
    }
    None
}

fn branch_index(doc: &UseCaseDocument, id: &str) -> Option<usize> {
    doc.branches.iter().position(|b| b.id == id)
}

/// The kind of element a list holds.
fn kind_of_list(at: &Location) -> ElementKind {
    match at {
        Location::Named(ListName::Preconditions) => ElementKind::Precondition,
        Location::Named(ListName::Postconditions) => ElementKind::Postcondition,
        _ => ElementKind::Step,
    }
}

/// The insertion index for `after` in `list`.
fn position(
    index: usize,
    list: &[Element],
    after: &Option<Option<String>>,
) -> Result<usize, OperationError> {
    match after {
        None => Ok(list.len()),
        Some(None) => Ok(0),
        Some(Some(id)) => match list.iter().position(|e| &e.id == id) {
            Some(i) => Ok(i + 1),
            None => Err(OperationError::Invalid {
                index,
                message: format!("\"{id}\" is not in the target list"),
            }),
        },
    }
}

struct Applier<'a> {
    doc: &'a mut UseCaseDocument,
    index: usize,
    effects: Vec<Effect>,
}

impl Applier<'_> {
    fn invalid<T>(&self, message: impl Into<String>) -> Result<T, OperationError> {
        Err(OperationError::Invalid {
            index: self.index,
            message: message.into(),
        })
    }

    fn not_found<T>(&self, id: &str) -> Result<T, OperationError> {
        Err(OperationError::NotFound {
            index: self.index,
            id: id.to_string(),
        })
    }

    fn resolve(&self, list: &ListRef) -> Result<Location, OperationError> {
        match list {
            ListRef::Named(name) => Ok(Location::Named(*name)),
            ListRef::Branch { branch } => match branch_index(self.doc, branch) {
                Some(i) => Ok(Location::Branch(i)),
                None => self.not_found(branch),
            },
        }
    }

    fn take_id(&mut self, given: Option<String>, kind: ElementKind) -> String {
        given.unwrap_or_else(|| {
            let id = new_id(kind);
            self.effects.push(Effect::Created { id: id.clone() });
            id
        })
    }

    fn main_step(&self, id: &str) -> Result<(), OperationError> {
        if self.doc.main.iter().any(|e| e.id == id) {
            Ok(())
        } else {
            self.invalid(format!("\"{id}\" is not a step of the main scenario"))
        }
    }

    fn apply(&mut self, op: Operation) -> Result<(), OperationError> {
        match op {
            Operation::SetHeader {
                title,
                primary_actor,
                secondary_actors,
                summary,
            } => {
                if let Some(title) = title {
                    self.doc.title = title;
                }
                if let Some(actor) = primary_actor {
                    self.doc.primary_actor = actor.filter(|a| !a.trim().is_empty());
                }
                if let Some(actors) = secondary_actors {
                    self.doc.secondary_actors = actors;
                }
                if let Some(summary) = summary {
                    self.doc.summary = summary.filter(|s| !s.trim().is_empty());
                }
            }
            Operation::AddElement {
                list,
                after,
                element,
            } => {
                let at = self.resolve(&list)?;
                let kind = kind_of_list(&at);
                let id = self.take_id(element.id, kind);
                if self.doc.contains(&id) {
                    return self.invalid(format!("id \"{id}\" already exists"));
                }
                let pos = position(self.index, list_mut(self.doc, &at), &after)?;
                list_mut(self.doc, &at).insert(
                    pos,
                    Element {
                        id,
                        text: element.text,
                    },
                );
            }
            Operation::UpdateText { id, text } => {
                let Some((at, i)) = find(self.doc, &id) else {
                    return self.not_found(&id);
                };
                list_mut(self.doc, &at)[i].text = text;
            }
            Operation::MoveElement { id, list, after } => {
                let Some((from, i)) = find(self.doc, &id) else {
                    return self.not_found(&id);
                };
                let to = match &list {
                    Some(list) => self.resolve(list)?,
                    None => from.clone(),
                };
                if kind_of_list(&to) != kind_of_list(&from) {
                    return self.invalid(format!("\"{id}\" cannot move to that list"));
                }
                if from == Location::Named(ListName::Main) && to != from {
                    let dependents = self.doc.branches_depending_on(&id);
                    if !dependents.is_empty() {
                        return Err(OperationError::HasDependents {
                            index: self.index,
                            id,
                            branches: dependents.into_iter().map(str::to_string).collect(),
                        });
                    }
                }
                if after.as_ref().and_then(|a| a.as_deref()) == Some(id.as_str()) {
                    return self.invalid("an element cannot move after itself");
                }
                let element = list_mut(self.doc, &from).remove(i);
                let pos = match position(self.index, list_mut(self.doc, &to), &after) {
                    Ok(pos) => pos,
                    Err(e) => {
                        list_mut(self.doc, &from).insert(i, element);
                        return Err(e);
                    }
                };
                list_mut(self.doc, &to).insert(pos, element);
            }
            Operation::RemoveElement { id, dependents } => {
                if find(self.doc, &id).is_none() {
                    return self.not_found(&id);
                }
                let depending: Vec<String> = self
                    .doc
                    .branches_depending_on(&id)
                    .into_iter()
                    .map(str::to_string)
                    .collect();
                if !depending.is_empty() {
                    match dependents {
                        None => {
                            return Err(OperationError::HasDependents {
                                index: self.index,
                                id,
                                branches: depending,
                            });
                        }
                        Some(Dependents::Cascade(_)) => {
                            self.doc.branches.retain(|b| !depending.contains(&b.id));
                            self.effects
                                .extend(depending.into_iter().map(|id| Effect::Removed { id }));
                        }
                        Some(Dependents::Rebranch { rebranch_to }) => {
                            if rebranch_to == id {
                                return self.invalid("cannot rebranch to the removed step");
                            }
                            self.main_step(&rebranch_to)?;
                            for branch in self.doc.branches.iter_mut() {
                                if !depending.contains(&branch.id) {
                                    continue;
                                }
                                if branch.from_step == id {
                                    branch.from_step = rebranch_to.clone();
                                }
                                if let BranchEnd::Rejoin { step } = &mut branch.end
                                    && *step == id
                                {
                                    *step = rebranch_to.clone();
                                }
                                self.effects.push(Effect::Rebranched {
                                    id: branch.id.clone(),
                                    from_step: branch.from_step.clone(),
                                });
                            }
                        }
                    }
                }
                // Cascading may have removed the branch holding the element.
                if let Some((at, i)) = find(self.doc, &id) {
                    list_mut(self.doc, &at).remove(i);
                }
            }
            Operation::AddBranch { branch } => {
                self.main_step(&branch.from_step)?;
                if let BranchEnd::Rejoin { step } = &branch.end {
                    self.main_step(step)?;
                }
                let id = self.take_id(branch.id, ElementKind::Branch);
                if self.doc.contains(&id) {
                    return self.invalid(format!("id \"{id}\" already exists"));
                }
                let mut steps = Vec::with_capacity(branch.steps.len());
                for step in branch.steps {
                    let step_id = self.take_id(step.id, ElementKind::Step);
                    steps.push(Element {
                        id: step_id,
                        text: step.text,
                    });
                }
                // Keep branches grouped by origin, in main-scenario order, so
                // the document reads like the labels.
                let origin = self.doc.main.iter().position(|e| e.id == branch.from_step);
                let pos = self
                    .doc
                    .branches
                    .iter()
                    .position(|b| self.doc.main.iter().position(|e| e.id == b.from_step) > origin)
                    .unwrap_or(self.doc.branches.len());
                self.doc.branches.insert(
                    pos,
                    Branch {
                        id,
                        kind: branch.kind,
                        title: branch.title,
                        from_step: branch.from_step,
                        steps,
                        end: branch.end,
                    },
                );
            }
            Operation::UpdateBranch {
                id,
                title,
                kind,
                from_step,
                end,
            } => {
                let Some(b) = branch_index(self.doc, &id) else {
                    return self.not_found(&id);
                };
                if let Some(step) = &from_step {
                    self.main_step(step)?;
                }
                if let Some(BranchEnd::Rejoin { step }) = &end {
                    self.main_step(step)?;
                }
                let branch = &mut self.doc.branches[b];
                if let Some(title) = title {
                    branch.title = title;
                }
                if let Some(kind) = kind {
                    branch.kind = kind;
                }
                if let Some(end) = end {
                    branch.end = end;
                }
                if let Some(step) = from_step
                    && step != branch.from_step
                {
                    branch.from_step = step.clone();
                    self.effects.push(Effect::Rebranched {
                        id: id.clone(),
                        from_step: step,
                    });
                }
            }
            Operation::RemoveBranch { id } => {
                let Some(b) = branch_index(self.doc, &id) else {
                    return self.not_found(&id);
                };
                self.doc.branches.remove(b);
            }
        }
        Ok(())
    }
}

/// Apply `ops` in order to `doc`. On error `doc` is left unchanged. The
/// effects include a `relabelled` entry for every branch whose label the
/// batch changed.
pub fn apply_operations(
    doc: &mut UseCaseDocument,
    ops: Vec<Operation>,
) -> Result<Vec<Effect>, OperationError> {
    let before = labels(doc);
    let mut work = doc.clone();
    let mut applier = Applier {
        doc: &mut work,
        index: 0,
        effects: Vec::new(),
    };
    for (index, op) in ops.into_iter().enumerate() {
        applier.index = index;
        applier.apply(op)?;
    }
    let mut effects = std::mem::take(&mut applier.effects);
    let after = labels(&work);
    for branch in &work.branches {
        if let (Some(from), Some(to)) = (before.get(&branch.id), after.get(&branch.id))
            && from != to
        {
            effects.push(Effect::Relabelled {
                id: branch.id.clone(),
                from: from.clone(),
                to: to.clone(),
            });
        }
    }
    *doc = work;
    Ok(effects)
}

#[cfg(test)]
mod tests {
    use super::super::tests::sample;
    use super::super::{Stage, validate};
    use super::*;
    use serde_json::json;

    fn ops(value: serde_json::Value) -> Vec<Operation> {
        serde_json::from_value(value).unwrap()
    }

    fn main_ids(doc: &UseCaseDocument) -> Vec<&str> {
        doc.main.iter().map(|e| e.id.as_str()).collect()
    }

    #[test]
    fn moving_a_step_relabels_the_branch_that_follows_it() {
        let mut doc = sample();
        let effects = apply_operations(
            &mut doc,
            ops(json!([{ "op": "move_element", "id": "s_C3mz", "after": "s_A1x9" }])),
        )
        .unwrap();
        assert_eq!(main_ids(&doc), ["s_A1x9", "s_C3mz", "s_Bf20", "s_D8pw"]);
        assert_eq!(labels(&doc)["b_N0s1"], "A2");
        assert_eq!(
            effects,
            vec![Effect::Relabelled {
                id: "b_N0s1".into(),
                from: "A3".into(),
                to: "A2".into()
            }]
        );
    }

    #[test]
    fn after_null_is_first_and_missing_is_last() {
        let mut doc = sample();
        apply_operations(
            &mut doc,
            ops(json!([
                { "op": "add_element", "list": "main", "after": null, "element": { "id": "s_First1", "text": "First" } },
                { "op": "add_element", "list": "main", "element": { "id": "s_Last1", "text": "Last" } }
            ])),
        )
        .unwrap();
        assert_eq!(doc.main.first().unwrap().id, "s_First1");
        assert_eq!(doc.main.last().unwrap().id, "s_Last1");
    }

    #[test]
    fn the_server_generates_missing_ids() {
        let mut doc = sample();
        let effects = apply_operations(
            &mut doc,
            ops(json!([{ "op": "add_element", "list": "preconditions", "element": { "text": "Online" } }])),
        )
        .unwrap();
        let Effect::Created { id } = &effects[0] else {
            panic!("{effects:?}")
        };
        assert!(id.starts_with("p_"));
        assert_eq!(doc.preconditions[1].id, *id);
    }

    #[test]
    fn removing_a_branch_origin_needs_a_decision() {
        let mut doc = sample();
        let err = apply_operations(
            &mut doc,
            ops(json!([{ "op": "remove_element", "id": "s_C3mz" }])),
        )
        .unwrap_err();
        assert_eq!(
            err,
            OperationError::HasDependents {
                index: 0,
                id: "s_C3mz".into(),
                branches: vec!["b_N0s1".into()]
            }
        );
        assert_eq!(
            doc,
            sample(),
            "a failed batch leaves the document unchanged"
        );

        let mut cascaded = sample();
        let effects = apply_operations(
            &mut cascaded,
            ops(json!([{ "op": "remove_element", "id": "s_C3mz", "dependents": "cascade" }])),
        )
        .unwrap();
        assert!(!cascaded.contains("b_N0s1") && !cascaded.contains("s_Q4ee"));
        assert!(effects.contains(&Effect::Removed {
            id: "b_N0s1".into()
        }));

        let mut moved = sample();
        apply_operations(
            &mut moved,
            ops(json!([{ "op": "remove_element", "id": "s_C3mz", "dependents": { "rebranch_to": "s_Bf20" } }])),
        )
        .unwrap();
        let branch = &moved.branches[0];
        assert_eq!(branch.from_step, "s_Bf20");
        assert_eq!(
            branch.end,
            BranchEnd::Rejoin {
                step: "s_Bf20".into()
            }
        );
        assert_eq!(labels(&moved)["b_N0s1"], "A2");
        assert!(validate(&moved, Stage::Publish).is_empty());
    }

    #[test]
    fn branches_are_added_in_origin_order() {
        let mut doc = sample();
        apply_operations(
            &mut doc,
            ops(json!([{ "op": "add_branch", "branch": {
                "id": "b_Early1", "kind": "exception", "title": "Page fails",
                "from_step": "s_A1x9", "end": { "type": "end", "outcome": "failure" },
                "steps": [{ "text": "System shows an error." }]
            } }])),
        )
        .unwrap();
        assert_eq!(doc.branches[0].id, "b_Early1");
        assert_eq!(labels(&doc)[&doc.branches[0].steps[0].id], "E1.1");
    }

    #[test]
    fn steps_move_between_main_and_a_branch() {
        let mut doc = sample();
        apply_operations(
            &mut doc,
            ops(json!([{ "op": "move_element", "id": "s_A1x9", "list": { "branch": "b_F4il" }, "after": null }])),
        )
        .unwrap();
        assert_eq!(doc.branches[1].steps[0].id, "s_A1x9");
        assert_eq!(doc.main.len(), 3);

        let err = apply_operations(
            &mut doc,
            ops(json!([{ "op": "move_element", "id": "s_C3mz", "list": { "branch": "b_F4il" } }])),
        )
        .unwrap_err();
        assert!(matches!(err, OperationError::HasDependents { .. }));

        let err = apply_operations(
            &mut doc,
            ops(json!([{ "op": "move_element", "id": "p_7Kq2", "list": "main" }])),
        )
        .unwrap_err();
        assert!(matches!(err, OperationError::Invalid { .. }));
    }

    #[test]
    fn branches_can_be_updated_and_removed() {
        let mut doc = sample();
        let effects = apply_operations(
            &mut doc,
            ops(json!([
                { "op": "update_branch", "id": "b_N0s1", "from_step": "s_Bf20", "title": "Empty selection" },
                { "op": "remove_branch", "id": "b_F4il" },
                { "op": "update_text", "id": "s_Q4ee", "text": "Ask again." },
                { "op": "set_header", "summary": null, "primary_actor": "Engineer" }
            ])),
        )
        .unwrap();
        assert_eq!(doc.branches.len(), 1);
        assert_eq!(doc.branches[0].title, "Empty selection");
        assert_eq!(doc.text_of("s_Q4ee"), Some("Ask again."));
        assert_eq!(doc.summary, None);
        assert_eq!(doc.primary_actor.as_deref(), Some("Engineer"));
        assert!(effects.contains(&Effect::Rebranched {
            id: "b_N0s1".into(),
            from_step: "s_Bf20".into()
        }));
        assert!(effects.contains(&Effect::Relabelled {
            id: "b_N0s1".into(),
            from: "A3".into(),
            to: "A2".into()
        }));
    }

    #[test]
    fn unknown_ids_and_fields_are_errors() {
        let mut doc = sample();
        let err = apply_operations(
            &mut doc,
            ops(json!([{ "op": "update_text", "id": "s_Nope1", "text": "x" }])),
        )
        .unwrap_err();
        assert!(matches!(err, OperationError::NotFound { .. }));
        assert!(
            serde_json::from_value::<Vec<Operation>>(
                json!([{ "op": "remove_branch", "id": "b_N0s1", "extra": 1 }])
            )
            .is_err()
        );
        assert!(serde_json::from_value::<Vec<Operation>>(json!([{ "op": "explode" }])).is_err());
    }
}
