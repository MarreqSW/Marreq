// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! The use case document, `marreq.use-case.v1` (issue #398, epic #388).
//!
//! A use case version stores one JSON document. Every element in it carries
//! a stable id (`p_` precondition, `q_` postcondition, `s_` step, `b_`
//! branch), so links, comments and diffs follow an element through
//! reordering. Display numbers (`3`, `A3`, `A3.1`, `E4`) are never stored:
//! [`labels`] computes them from order and branch origin.

pub mod diff;
pub mod operations;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

pub use operations::{Effect, Operation, OperationError, apply_operations};

pub const SCHEMA: &str = "marreq.use-case.v1";

pub const TITLE_MIN: usize = 3;
pub const TITLE_MAX: usize = 255;
pub const TEXT_MAX: usize = 2000;
pub const ACTOR_MAX: usize = 255;
pub const SECONDARY_ACTORS_MAX: usize = 20;
pub const ELEMENTS_MAX: usize = 500;

static ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[pqsb]_[A-Za-z0-9]{4,32}$").unwrap());

fn schema_v1() -> String {
    SCHEMA.to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UseCaseDocument {
    #[serde(default = "schema_v1")]
    pub schema: String,
    pub title: String,
    #[serde(default)]
    pub primary_actor: Option<String>,
    #[serde(default)]
    pub secondary_actors: Vec<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub preconditions: Vec<Element>,
    #[serde(default)]
    pub main: Vec<Element>,
    #[serde(default)]
    pub branches: Vec<Branch>,
    #[serde(default)]
    pub postconditions: Vec<Element>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Element {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchKind {
    Alternative,
    Exception,
}

impl BranchKind {
    fn letter(self) -> char {
        match self {
            BranchKind::Alternative => 'A',
            BranchKind::Exception => 'E',
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Success,
    Failure,
}

/// Where a branch goes after its last step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum BranchEnd {
    /// Back to a step of the main scenario.
    Rejoin { step: String },
    /// The use case ends here.
    End { outcome: Outcome },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Branch {
    pub id: String,
    pub kind: BranchKind,
    pub title: String,
    /// The main step this branch starts from.
    pub from_step: String,
    #[serde(default)]
    pub steps: Vec<Element>,
    pub end: BranchEnd,
}

/// The kind of an element, from its id prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ElementKind {
    Precondition,
    Postcondition,
    Step,
    Branch,
}

impl ElementKind {
    pub fn of(id: &str) -> Option<Self> {
        match id.split_once('_')?.0 {
            "p" => Some(ElementKind::Precondition),
            "q" => Some(ElementKind::Postcondition),
            "s" => Some(ElementKind::Step),
            "b" => Some(ElementKind::Branch),
            _ => None,
        }
    }

    pub fn prefix(self) -> &'static str {
        match self {
            ElementKind::Precondition => "p",
            ElementKind::Postcondition => "q",
            ElementKind::Step => "s",
            ElementKind::Branch => "b",
        }
    }
}

/// A new random element id, such as `s_k3Q9xT0a`.
pub fn new_id(kind: ElementKind) -> String {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut n: u64 = rand::random();
    let mut out = String::with_capacity(10);
    out.push_str(kind.prefix());
    out.push('_');
    for _ in 0..8 {
        out.push(ALPHABET[(n % 62) as usize] as char);
        n /= 62;
    }
    out
}

impl UseCaseDocument {
    /// An empty document with a title, as created with a new use case.
    pub fn new(title: &str, primary_actor: Option<&str>) -> Self {
        Self {
            schema: SCHEMA.to_string(),
            title: title.to_string(),
            primary_actor: primary_actor.map(str::to_string),
            secondary_actors: Vec::new(),
            summary: None,
            preconditions: Vec::new(),
            main: Vec::new(),
            branches: Vec::new(),
            postconditions: Vec::new(),
        }
    }

    /// Every element id in the document, in document order.
    pub fn ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = Vec::new();
        ids.extend(self.preconditions.iter().map(|e| e.id.as_str()));
        ids.extend(self.main.iter().map(|e| e.id.as_str()));
        for branch in &self.branches {
            ids.push(&branch.id);
            ids.extend(branch.steps.iter().map(|e| e.id.as_str()));
        }
        ids.extend(self.postconditions.iter().map(|e| e.id.as_str()));
        ids
    }

    pub fn contains(&self, id: &str) -> bool {
        self.ids().contains(&id)
    }

    pub fn element_count(&self) -> usize {
        self.ids().len()
    }

    /// Steps in the main scenario and in every branch.
    pub fn step_count(&self) -> usize {
        self.main.len() + self.branches.iter().map(|b| b.steps.len()).sum::<usize>()
    }

    pub fn branch_count(&self, kind: BranchKind) -> usize {
        self.branches.iter().filter(|b| b.kind == kind).count()
    }

    /// The text of an element, or a branch's title.
    pub fn text_of(&self, id: &str) -> Option<&str> {
        self.preconditions
            .iter()
            .chain(&self.main)
            .chain(&self.postconditions)
            .chain(self.branches.iter().flat_map(|b| &b.steps))
            .find(|e| e.id == id)
            .map(|e| e.text.as_str())
            .or_else(|| {
                self.branches
                    .iter()
                    .find(|b| b.id == id)
                    .map(|b| b.title.as_str())
            })
    }

    /// Branches that start from or rejoin at `step_id`.
    pub fn branches_depending_on(&self, step_id: &str) -> Vec<&str> {
        self.branches
            .iter()
            .filter(|b| {
                b.from_step == step_id
                    || matches!(&b.end, BranchEnd::Rejoin { step } if step == step_id)
            })
            .map(|b| b.id.as_str())
            .collect()
    }

    /// Text used to search a version: header, then every element's text.
    pub fn search_text(&self) -> String {
        let mut parts: Vec<&str> = vec![&self.title];
        parts.extend(self.primary_actor.as_deref());
        parts.extend(self.secondary_actors.iter().map(String::as_str));
        parts.extend(self.summary.as_deref());
        for id in self.ids() {
            parts.extend(self.text_of(id));
        }
        parts.join("\n")
    }
}

/// One broken invariant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DocumentError {
    /// Where, as a JSON-ish path (`main[2].text`) or an element id.
    pub path: String,
    pub message: String,
}

impl DocumentError {
    fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

/// When the document is checked: drafts may be incomplete, a published
/// version may not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Draft,
    Publish,
}

fn check_text(errors: &mut Vec<DocumentError>, path: String, text: &str) {
    let len = text.trim().chars().count();
    if len == 0 {
        errors.push(DocumentError::new(path, "must not be empty"));
    } else if text.chars().count() > TEXT_MAX {
        errors.push(DocumentError::new(
            path,
            format!("must be at most {TEXT_MAX} characters"),
        ));
    }
}

/// Check every invariant of the format. An empty list means the document is
/// valid.
pub fn validate(doc: &UseCaseDocument, stage: Stage) -> Vec<DocumentError> {
    let mut errors = Vec::new();

    if doc.schema != SCHEMA {
        errors.push(DocumentError::new(
            "schema",
            format!("must be \"{SCHEMA}\""),
        ));
    }
    let title_len = doc.title.trim().chars().count();
    if !(TITLE_MIN..=TITLE_MAX).contains(&title_len) {
        errors.push(DocumentError::new(
            "title",
            format!("must be {TITLE_MIN} to {TITLE_MAX} characters"),
        ));
    }
    if let Some(actor) = &doc.primary_actor
        && actor.chars().count() > ACTOR_MAX
    {
        errors.push(DocumentError::new(
            "primary_actor",
            format!("must be at most {ACTOR_MAX} characters"),
        ));
    }
    if doc.secondary_actors.len() > SECONDARY_ACTORS_MAX {
        errors.push(DocumentError::new(
            "secondary_actors",
            format!("at most {SECONDARY_ACTORS_MAX} actors"),
        ));
    }
    for (i, actor) in doc.secondary_actors.iter().enumerate() {
        let len = actor.trim().chars().count();
        if len == 0 || len > ACTOR_MAX {
            errors.push(DocumentError::new(
                format!("secondary_actors[{i}]"),
                format!("must be 1 to {ACTOR_MAX} characters"),
            ));
        }
    }
    if let Some(summary) = &doc.summary
        && summary.chars().count() > TEXT_MAX
    {
        errors.push(DocumentError::new(
            "summary",
            format!("must be at most {TEXT_MAX} characters"),
        ));
    }

    // Ids: well formed, of the right kind for their list, unique.
    let mut seen: HashSet<String> = HashSet::new();
    let mut check_id =
        |errors: &mut Vec<DocumentError>, path: String, id: &str, kind: ElementKind| {
            if !ID_RE.is_match(id) {
                errors.push(DocumentError::new(path, format!("invalid id \"{id}\"")));
            } else if ElementKind::of(id) != Some(kind) {
                errors.push(DocumentError::new(
                    path,
                    format!("id \"{id}\" must start with \"{}_\"", kind.prefix()),
                ));
            }
            if !seen.insert(id.to_string()) {
                errors.push(DocumentError::new(id.to_string(), "duplicate id"));
            }
        };
    let lists = [
        (
            "preconditions",
            &doc.preconditions,
            ElementKind::Precondition,
        ),
        ("main", &doc.main, ElementKind::Step),
        (
            "postconditions",
            &doc.postconditions,
            ElementKind::Postcondition,
        ),
    ];
    for (name, list, kind) in lists {
        for (i, element) in list.iter().enumerate() {
            check_id(&mut errors, format!("{name}[{i}].id"), &element.id, kind);
            check_text(&mut errors, format!("{name}[{i}].text"), &element.text);
        }
    }
    let main_ids: HashSet<&str> = doc.main.iter().map(|e| e.id.as_str()).collect();
    for (i, branch) in doc.branches.iter().enumerate() {
        let path = format!("branches[{i}]");
        check_id(
            &mut errors,
            format!("{path}.id"),
            &branch.id,
            ElementKind::Branch,
        );
        let title_len = branch.title.trim().chars().count();
        if title_len == 0 || title_len > TITLE_MAX {
            errors.push(DocumentError::new(
                format!("{path}.title"),
                format!("must be 1 to {TITLE_MAX} characters"),
            ));
        }
        if !main_ids.contains(branch.from_step.as_str()) {
            errors.push(DocumentError::new(
                format!("{path}.from_step"),
                "must be a step of the main scenario",
            ));
        }
        if let BranchEnd::Rejoin { step } = &branch.end
            && !main_ids.contains(step.as_str())
        {
            errors.push(DocumentError::new(
                format!("{path}.end.step"),
                "must be a step of the main scenario",
            ));
        }
        for (j, step) in branch.steps.iter().enumerate() {
            check_id(
                &mut errors,
                format!("{path}.steps[{j}].id"),
                &step.id,
                ElementKind::Step,
            );
            check_text(&mut errors, format!("{path}.steps[{j}].text"), &step.text);
        }
    }

    let count = doc.element_count();
    if count > ELEMENTS_MAX {
        errors.push(DocumentError::new(
            "document",
            format!("at most {ELEMENTS_MAX} elements ({count})"),
        ));
    }
    if stage == Stage::Publish && doc.main.is_empty() {
        errors.push(DocumentError::new(
            "main",
            "the main scenario needs at least one step",
        ));
    }
    errors
}

/// Display labels for steps and branches: main steps `1…n`; a branch is `A`
/// or `E` plus its origin's number (`A3`, then `A3b`, `A3c` for further
/// branches of the same kind from the same step); branch steps are
/// `<branch>.<n>`. Preconditions and postconditions are `P1…`, `Q1…`.
pub fn labels(doc: &UseCaseDocument) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    for (i, e) in doc.preconditions.iter().enumerate() {
        labels.insert(e.id.clone(), format!("P{}", i + 1));
    }
    for (i, e) in doc.postconditions.iter().enumerate() {
        labels.insert(e.id.clone(), format!("Q{}", i + 1));
    }
    let mut main_number: HashMap<&str, usize> = HashMap::new();
    for (i, e) in doc.main.iter().enumerate() {
        main_number.insert(&e.id, i + 1);
        labels.insert(e.id.clone(), (i + 1).to_string());
    }
    let mut seen: HashMap<(char, &str), usize> = HashMap::new();
    for branch in &doc.branches {
        let letter = branch.kind.letter();
        let origin = main_number
            .get(branch.from_step.as_str())
            .map(|n| n.to_string())
            .unwrap_or_else(|| "?".into());
        let nth = seen.entry((letter, &branch.from_step)).or_insert(0);
        *nth += 1;
        let suffix = if *nth == 1 {
            String::new()
        } else {
            // b, c, … z, then aa-style numbers are not needed below 25.
            char::from(b'a' + (*nth as u8 - 1).min(25)).to_string()
        };
        let label = format!("{letter}{origin}{suffix}");
        for (j, step) in branch.steps.iter().enumerate() {
            labels.insert(step.id.clone(), format!("{label}.{}", j + 1));
        }
        labels.insert(branch.id.clone(), label);
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    pub(crate) fn sample() -> UseCaseDocument {
        serde_json::from_value(json!({
            "schema": SCHEMA,
            "title": "Export requirements",
            "primary_actor": "Project member",
            "secondary_actors": ["Storage service"],
            "summary": "A member exports a selection of requirements to a file.",
            "preconditions": [{ "id": "p_7Kq2", "text": "User is authenticated." }],
            "main": [
                { "id": "s_A1x9", "text": "User opens the requirements page." },
                { "id": "s_Bf20", "text": "System displays the available requirements." },
                { "id": "s_C3mz", "text": "User selects requirements and requests an export." },
                { "id": "s_D8pw", "text": "System generates the file in the chosen format." }
            ],
            "branches": [
                {
                    "id": "b_N0s1", "kind": "alternative", "title": "Nothing selected",
                    "from_step": "s_C3mz",
                    "steps": [{ "id": "s_Q4ee", "text": "System asks the user to select at least one requirement." }],
                    "end": { "type": "rejoin", "step": "s_C3mz" }
                },
                {
                    "id": "b_F4il", "kind": "exception", "title": "Export fails",
                    "from_step": "s_D8pw",
                    "steps": [{ "id": "s_R7kk", "text": "System reports the error and keeps the selection." }],
                    "end": { "type": "end", "outcome": "failure" }
                }
            ],
            "postconditions": [{ "id": "q_2Lm0", "text": "The export file is available to the user." }]
        }))
        .unwrap()
    }

    #[test]
    fn the_rfc_example_is_valid_and_round_trips() {
        let doc = sample();
        assert!(validate(&doc, Stage::Publish).is_empty());
        let back: UseCaseDocument =
            serde_json::from_value(serde_json::to_value(&doc).unwrap()).unwrap();
        assert_eq!(back, doc);
    }

    #[test]
    fn labels_follow_order_and_origin() {
        let labels = labels(&sample());
        assert_eq!(labels["s_C3mz"], "3");
        assert_eq!(labels["b_N0s1"], "A3");
        assert_eq!(labels["s_Q4ee"], "A3.1");
        assert_eq!(labels["b_F4il"], "E4");
        assert_eq!(labels["s_R7kk"], "E4.1");
        assert_eq!(labels["p_7Kq2"], "P1");
        assert_eq!(labels["q_2Lm0"], "Q1");
    }

    #[test]
    fn a_second_branch_of_the_same_kind_gets_a_letter() {
        let mut doc = sample();
        let mut second = doc.branches[0].clone();
        second.id = "b_Two2".into();
        second.steps.clear();
        doc.branches.push(second);
        let labels = labels(&doc);
        assert_eq!(labels["b_N0s1"], "A3");
        assert_eq!(labels["b_Two2"], "A3b");
    }

    #[test]
    fn unknown_keys_are_rejected() {
        let mut value = serde_json::to_value(sample()).unwrap();
        value["colour"] = json!("red");
        assert!(serde_json::from_value::<UseCaseDocument>(value).is_err());
        let mut value = serde_json::to_value(sample()).unwrap();
        value["main"][0]["note"] = json!("x");
        assert!(serde_json::from_value::<UseCaseDocument>(value).is_err());
    }

    #[test]
    fn broken_invariants_are_reported() {
        let mut doc = sample();
        doc.branches[0].from_step = "s_R7kk".into(); // a branch step, not a main one
        doc.main[1].id = "p_Bf20".into(); // wrong kind for the list
        doc.main[2].text = "  ".into();
        doc.preconditions.push(Element {
            id: "s_A1x9".into(), // duplicate, and wrong kind
            text: "Again".into(),
        });
        let errors = validate(&doc, Stage::Draft);
        let paths: Vec<&str> = errors.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"branches[0].from_step"), "{errors:?}");
        assert!(paths.contains(&"main[1].id"), "{errors:?}");
        assert!(paths.contains(&"main[2].text"), "{errors:?}");
        assert!(paths.contains(&"s_A1x9"), "{errors:?}");
        assert!(paths.contains(&"preconditions[1].id"), "{errors:?}");
    }

    #[test]
    fn an_empty_main_scenario_is_a_draft_only() {
        let doc = UseCaseDocument::new("Export requirements", None);
        assert!(validate(&doc, Stage::Draft).is_empty());
        assert_eq!(validate(&doc, Stage::Publish).len(), 1);
    }

    #[test]
    fn the_title_length_is_checked() {
        let doc = UseCaseDocument::new("ab", None);
        assert_eq!(validate(&doc, Stage::Draft)[0].path, "title");
    }

    #[test]
    fn new_ids_are_valid_and_of_their_kind() {
        for kind in [ElementKind::Step, ElementKind::Branch] {
            let id = new_id(kind);
            assert!(ID_RE.is_match(&id), "{id}");
            assert_eq!(ElementKind::of(&id), Some(kind));
        }
        assert_ne!(new_id(ElementKind::Step), new_id(ElementKind::Step));
    }

    #[test]
    fn dependents_and_counts() {
        let doc = sample();
        assert_eq!(doc.branches_depending_on("s_C3mz"), vec!["b_N0s1"]);
        assert!(doc.branches_depending_on("s_A1x9").is_empty());
        assert_eq!(doc.step_count(), 6);
        assert_eq!(doc.branch_count(BranchKind::Exception), 1);
        assert_eq!(doc.text_of("b_F4il"), Some("Export fails"));
        assert!(doc.search_text().contains("keeps the selection"));
    }
}
