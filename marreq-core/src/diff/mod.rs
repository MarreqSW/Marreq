// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Deterministic diffs between requirement versions and verification snapshots.
//!
//! Text fields use line-based diff; metadata fields are compared for
//! added/removed/unchanged values. Read-only and audit-safe.

use crate::models::{CustomFieldValueDisplay, RequirementVersion};
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};
use std::collections::{BTreeMap, HashSet};

/// Line-based text diff result: added, removed, and unchanged lines.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TextDiffResult {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub unchanged: Vec<String>,
}

/// Single metadata field diff (e.g. status, category, applicability).
/// When unchanged, old_id == new_id and both are Some.
/// Optional _label fields are human-readable titles (resolved by the service for display).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleValueDiff {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_id: Option<i32>,
    /// Present when value did not change (old_id == new_id).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unchanged: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unchanged_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_label: Option<String>,
}

/// Verification methods diff: sets of IDs added, removed, unchanged.
/// Optional _labels vecs match order of _ids (resolved by the service for display).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerificationDiff {
    pub added_ids: Vec<i32>,
    pub removed_ids: Vec<i32>,
    pub unchanged_ids: Vec<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub added_labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed_labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unchanged_labels: Option<Vec<String>>,
}

/// Metadata section of the requirement diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataDiff {
    pub status: SingleValueDiff,
    pub category: SingleValueDiff,
    pub applicability: SingleValueDiff,
    pub verification: VerificationDiff,
    #[serde(default)]
    pub custom_fields: Vec<CustomFieldDiff>,
}

/// One version-scoped custom field comparison.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomFieldDiff {
    pub field_id: i32,
    pub label: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub unchanged: bool,
}

/// Full structured diff between two requirement versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementDiff {
    pub text: TextDiffSection,
    pub metadata: MetadataDiff,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDiffSection {
    pub title: TextDiffResult,
    pub description: TextDiffResult,
    pub justification: TextDiffResult,
}

/// Snapshot of a verification used for diffs (from a version row or an audit log payload).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationSnapshotState {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub reference_code: String,
    #[serde(default)]
    pub status_id: i32,
    pub parent_id: Option<i32>,
    pub verification_method_id: Option<i32>,
}

/// Metadata section of a verification snapshot diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMetadataDiff {
    pub status: SingleValueDiff,
    pub verification_method: SingleValueDiff,
    pub parent: SingleValueDiff,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationTextDiffSection {
    pub name: TextDiffResult,
    pub description: TextDiffResult,
    pub source: TextDiffResult,
    pub reference_code: TextDiffResult,
}

/// Full structured diff between two verification snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationVersionDiff {
    pub text: VerificationTextDiffSection,
    pub metadata: VerificationMetadataDiff,
}

/// Compute a deterministic diff between two verification snapshots (v1 = old, v2 = new).
pub fn compute_verification_diff(
    v1: &VerificationSnapshotState,
    v2: &VerificationSnapshotState,
) -> VerificationVersionDiff {
    VerificationVersionDiff {
        text: VerificationTextDiffSection {
            name: line_diff(&v1.name, &v2.name),
            description: line_diff(&v1.description, &v2.description),
            source: line_diff(&v1.source, &v2.source),
            reference_code: line_diff(&v1.reference_code, &v2.reference_code),
        },
        metadata: VerificationMetadataDiff {
            status: single_value_diff(v1.status_id, v2.status_id),
            verification_method: optional_id_diff(
                v1.verification_method_id,
                v2.verification_method_id,
            ),
            parent: optional_id_diff(v1.parent_id, v2.parent_id),
        },
    }
}

/// Compute a deterministic diff between two requirement versions (v1 = old, v2 = new).
/// Verification IDs must be sorted (as returned by the repository).
pub fn compute_requirement_diff(
    v1: &RequirementVersion,
    v2: &RequirementVersion,
    verification_v1: &[i32],
    verification_v2: &[i32],
) -> RequirementDiff {
    RequirementDiff {
        text: TextDiffSection {
            title: line_diff(&v1.title, &v2.title),
            description: line_diff(&v1.description, &v2.description),
            justification: line_diff(
                v1.justification.as_deref().unwrap_or_default(),
                v2.justification.as_deref().unwrap_or_default(),
            ),
        },
        metadata: MetadataDiff {
            status: single_value_diff(v1.status_id, v2.status_id),
            category: single_value_diff(v1.category_id, v2.category_id),
            applicability: single_value_diff(v1.applicability_id, v2.applicability_id),
            verification: verification_diff(verification_v1, verification_v2),
            custom_fields: Vec::new(),
        },
    }
}

/// Compare custom field values from two immutable requirement versions.
///
/// A `BTreeMap` keeps the response stable by field id. Labels come from the
/// newer version when possible, while fields removed from the newer snapshot
/// retain their historical label from the older version.
pub fn custom_field_diff(
    old_values: &[CustomFieldValueDisplay],
    new_values: &[CustomFieldValueDisplay],
) -> Vec<CustomFieldDiff> {
    let mut fields: BTreeMap<i32, (String, Option<String>, Option<String>)> = BTreeMap::new();

    for field in old_values {
        fields.insert(
            field.field_id,
            (field.label.clone(), field.value.clone(), None),
        );
    }
    for field in new_values {
        fields
            .entry(field.field_id)
            .and_modify(|entry| {
                entry.0 = field.label.clone();
                entry.2 = field.value.clone();
            })
            .or_insert_with(|| (field.label.clone(), None, field.value.clone()));
    }

    fields
        .into_iter()
        .map(
            |(field_id, (label, old_value, new_value))| CustomFieldDiff {
                field_id,
                label,
                unchanged: old_value == new_value,
                old_value,
                new_value,
            },
        )
        .collect()
}

/// Line-based diff: split on newline, then classify each line as added, removed, or unchanged.
fn line_diff(old_str: &str, new_str: &str) -> TextDiffResult {
    let old_lines: Vec<&str> = old_str.split('\n').collect();
    let new_lines: Vec<&str> = new_str.split('\n').collect();
    let diff = TextDiff::from_slices(&old_lines, &new_lines);

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut unchanged = Vec::new();

    for change in diff.iter_all_changes() {
        let line = change.to_string().trim_end_matches('\n').to_string();
        match change.tag() {
            ChangeTag::Equal => unchanged.push(line),
            ChangeTag::Delete => removed.push(line),
            ChangeTag::Insert => added.push(line),
        }
    }

    TextDiffResult {
        added,
        removed,
        unchanged,
    }
}

fn optional_id_diff(old_id: Option<i32>, new_id: Option<i32>) -> SingleValueDiff {
    if old_id == new_id {
        SingleValueDiff {
            old_id,
            new_id,
            unchanged: old_id,
            unchanged_label: None,
            old_label: None,
            new_label: None,
        }
    } else {
        SingleValueDiff {
            old_id,
            new_id,
            unchanged: None,
            unchanged_label: None,
            old_label: None,
            new_label: None,
        }
    }
}

fn single_value_diff(old_id: i32, new_id: i32) -> SingleValueDiff {
    if old_id == new_id {
        SingleValueDiff {
            old_id: Some(old_id),
            new_id: Some(new_id),
            unchanged: Some(old_id),
            unchanged_label: None,
            old_label: None,
            new_label: None,
        }
    } else {
        SingleValueDiff {
            old_id: Some(old_id),
            new_id: Some(new_id),
            unchanged: None,
            unchanged_label: None,
            old_label: None,
            new_label: None,
        }
    }
}

/// Compare two sorted slices of verification method IDs.
fn verification_diff(v1: &[i32], v2: &[i32]) -> VerificationDiff {
    let set1: HashSet<i32> = v1.iter().copied().collect();
    let set2: HashSet<i32> = v2.iter().copied().collect();

    let mut added_ids: Vec<i32> = set2.difference(&set1).copied().collect();
    let mut removed_ids: Vec<i32> = set1.difference(&set2).copied().collect();
    let mut unchanged_ids: Vec<i32> = set1.intersection(&set2).copied().collect();

    added_ids.sort_unstable();
    removed_ids.sort_unstable();
    unchanged_ids.sort_unstable();

    VerificationDiff {
        added_ids,
        removed_ids,
        unchanged_ids,
        added_labels: None,
        removed_labels: None,
        unchanged_labels: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn naive_dt(days: i64) -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            + chrono::Duration::days(days)
    }

    fn version_with_text(
        id: i32,
        req_id: i32,
        title: &str,
        description: &str,
        status_id: i32,
        category_id: i32,
        applicability_id: i32,
    ) -> RequirementVersion {
        RequirementVersion {
            id,
            requirement_id: req_id,
            title: title.to_string(),
            description: description.to_string(),
            status_id,
            author_id: 1,
            reviewer_id: 1,
            category_id,
            applicability_id,
            justification: None,
            deadline_date: None,
            created_at: naive_dt(id as i64),
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            reviewed_by: None,
            reviewed_at: None,
        }
    }

    #[test]
    fn text_diff_added_removed_unchanged() {
        let mut v1 = version_with_text(1, 1, "Line A\nLine B", "Desc 1", 1, 1, 1);
        v1.justification = Some("Old rationale".to_string());
        let mut v2 = version_with_text(2, 1, "Line A\nLine C\nLine B", "Desc 1\nNew line", 1, 1, 1);
        v2.justification = Some("New rationale".to_string());
        let diff = compute_requirement_diff(&v1, &v2, &[], &[]);

        assert_eq!(diff.text.title.unchanged, ["Line A", "Line B"]);
        assert_eq!(diff.text.title.added, ["Line C"]);
        assert_eq!(diff.text.title.removed.len(), 0);

        assert_eq!(diff.text.description.unchanged, ["Desc 1"]);
        assert_eq!(diff.text.description.added, ["New line"]);
        assert_eq!(diff.text.description.removed.len(), 0);
        assert_eq!(diff.text.justification.removed, ["Old rationale"]);
        assert_eq!(diff.text.justification.added, ["New rationale"]);
    }

    #[test]
    fn text_diff_deterministic() {
        let v1 = version_with_text(1, 1, "a\nb\nc", "x\ny", 1, 1, 1);
        let v2 = version_with_text(2, 1, "a\nb2\nc", "x\nz\ny", 1, 1, 1);
        let d1 = compute_requirement_diff(&v1, &v2, &[1], &[1, 2]);
        let d2 = compute_requirement_diff(&v1, &v2, &[1], &[1, 2]);
        assert_eq!(
            serde_json::to_string(&d1).unwrap(),
            serde_json::to_string(&d2).unwrap()
        );
    }

    #[test]
    fn metadata_single_value_unchanged() {
        let v1 = version_with_text(1, 1, "T", "D", 5, 10, 20);
        let v2 = version_with_text(2, 1, "T", "D", 5, 10, 20);
        let diff = compute_requirement_diff(&v1, &v2, &[1, 2], &[1, 2]);
        assert_eq!(diff.metadata.status.unchanged, Some(5));
        assert_eq!(diff.metadata.category.unchanged, Some(10));
        assert_eq!(diff.metadata.applicability.unchanged, Some(20));
        assert!(diff.metadata.verification.added_ids.is_empty());
        assert!(diff.metadata.verification.removed_ids.is_empty());
        assert_eq!(diff.metadata.verification.unchanged_ids, [1, 2]);
    }

    #[test]
    fn metadata_single_value_changed() {
        let v1 = version_with_text(1, 1, "T", "D", 1, 1, 1);
        let v2 = version_with_text(2, 1, "T", "D", 2, 3, 4);
        let diff = compute_requirement_diff(&v1, &v2, &[], &[]);
        assert_eq!(diff.metadata.status.old_id, Some(1));
        assert_eq!(diff.metadata.status.new_id, Some(2));
        assert_eq!(diff.metadata.status.unchanged, None);
        assert_eq!(diff.metadata.category.old_id, Some(1));
        assert_eq!(diff.metadata.category.new_id, Some(3));
        assert_eq!(diff.metadata.applicability.old_id, Some(1));
        assert_eq!(diff.metadata.applicability.new_id, Some(4));
    }

    #[test]
    fn verification_diff_added_removed_unchanged() {
        let v1 = version_with_text(1, 1, "T", "D", 1, 1, 1);
        let v2 = version_with_text(2, 1, "T", "D", 1, 1, 1);
        let diff = compute_requirement_diff(&v1, &v2, &[1, 3, 5], &[1, 2, 5]);
        assert_eq!(diff.metadata.verification.added_ids, [2]);
        assert_eq!(diff.metadata.verification.removed_ids, [3]);
        assert_eq!(diff.metadata.verification.unchanged_ids, [1, 5]);
    }

    #[test]
    fn custom_fields_are_compared_in_stable_field_id_order() {
        let old = vec![
            CustomFieldValueDisplay {
                field_id: 8,
                label: "Owner".to_string(),
                value: Some("Power".to_string()),
            },
            CustomFieldValueDisplay {
                field_id: 2,
                label: "Critical".to_string(),
                value: Some("true".to_string()),
            },
        ];
        let new = vec![
            CustomFieldValueDisplay {
                field_id: 8,
                label: "Subsystem owner".to_string(),
                value: Some("Avionics".to_string()),
            },
            CustomFieldValueDisplay {
                field_id: 5,
                label: "Margin".to_string(),
                value: Some("20".to_string()),
            },
            CustomFieldValueDisplay {
                field_id: 2,
                label: "Critical".to_string(),
                value: Some("true".to_string()),
            },
        ];

        let fields = custom_field_diff(&old, &new);

        assert_eq!(
            fields
                .iter()
                .map(|field| field.field_id)
                .collect::<Vec<_>>(),
            [2, 5, 8]
        );
        assert!(fields[0].unchanged);
        assert_eq!(fields[1].old_value, None);
        assert_eq!(fields[1].new_value.as_deref(), Some("20"));
        assert_eq!(fields[2].label, "Subsystem owner");
        assert_eq!(fields[2].old_value.as_deref(), Some("Power"));
        assert_eq!(fields[2].new_value.as_deref(), Some("Avionics"));
        assert!(!fields[2].unchanged);
    }

    fn verification_state(
        name: &str,
        description: &str,
        source: &str,
        reference: &str,
        status_id: i32,
        method_id: Option<i32>,
        parent_id: Option<i32>,
    ) -> VerificationSnapshotState {
        VerificationSnapshotState {
            name: name.into(),
            description: description.into(),
            source: source.into(),
            reference_code: reference.into(),
            status_id,
            parent_id,
            verification_method_id: method_id,
        }
    }

    #[test]
    fn verification_text_and_metadata_diff() {
        let v1 = verification_state(
            "Power test",
            "Measure 500W",
            "TV-001",
            "VER-001",
            1,
            Some(1),
            None,
        );
        let v2 = verification_state(
            "Power test",
            "Measure 650W\nAt SAR output",
            "TV-001 rev B",
            "VER-001",
            2,
            Some(3),
            Some(9),
        );
        let diff = compute_verification_diff(&v1, &v2);
        assert_eq!(diff.text.name.unchanged, ["Power test"]);
        assert_eq!(diff.text.description.removed, ["Measure 500W"]);
        assert_eq!(
            diff.text.description.added,
            ["Measure 650W", "At SAR output"]
        );
        assert_eq!(diff.text.source.removed, ["TV-001"]);
        assert_eq!(diff.text.source.added, ["TV-001 rev B"]);
        assert_eq!(diff.text.reference_code.unchanged, ["VER-001"]);
        assert_eq!(diff.metadata.status.old_id, Some(1));
        assert_eq!(diff.metadata.status.new_id, Some(2));
        assert_eq!(diff.metadata.verification_method.old_id, Some(1));
        assert_eq!(diff.metadata.verification_method.new_id, Some(3));
        assert_eq!(diff.metadata.parent.old_id, None);
        assert_eq!(diff.metadata.parent.new_id, Some(9));
    }

    #[test]
    fn optional_ids_unchanged_when_both_absent() {
        let v1 = verification_state("N", "D", "S", "R", 1, None, None);
        let v2 = verification_state("N", "D", "S", "R", 1, None, None);
        let diff = compute_verification_diff(&v1, &v2);
        assert_eq!(diff.metadata.verification_method.unchanged, None);
        assert_eq!(diff.metadata.verification_method.old_id, None);
        assert_eq!(diff.metadata.verification_method.new_id, None);
        assert_eq!(diff.metadata.parent.unchanged, None);
    }
}
