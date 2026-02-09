//! Deterministic diff between two requirement versions.
//!
//! Text fields (title, description) use line-based diff; metadata (status, category,
//! applicability, verification) are compared for added/removed/unchanged. Read-only and audit-safe.

use crate::models::RequirementVersion;
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};

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
        },
        metadata: MetadataDiff {
            status: single_value_diff(v1.status_id, v2.status_id),
            category: single_value_diff(v1.category_id, v2.category_id),
            applicability: single_value_diff(v1.applicability_id, v2.applicability_id),
            verification: verification_diff(verification_v1, verification_v2),
        },
    }
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
    let set1: std::collections::HashSet<i32> = v1.iter().copied().collect();
    let set2: std::collections::HashSet<i32> = v2.iter().copied().collect();

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
            parent_id: None,
            applicability_id,
            justification: None,
            deadline_date: None,
            created_at: naive_dt(id as i64),
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
        }
    }

    #[test]
    fn text_diff_added_removed_unchanged() {
        let v1 = version_with_text(1, 1, "Line A\nLine B", "Desc 1", 1, 1, 1);
        let v2 = version_with_text(2, 1, "Line A\nLine C\nLine B", "Desc 1\nNew line", 1, 1, 1);
        let diff = compute_requirement_diff(&v1, &v2, &[], &[]);

        assert_eq!(diff.text.title.unchanged, ["Line A", "Line B"]);
        assert_eq!(diff.text.title.added, ["Line C"]);
        assert_eq!(diff.text.title.removed.len(), 0);

        assert_eq!(diff.text.description.unchanged, ["Desc 1"]);
        assert_eq!(diff.text.description.added, ["New line"]);
        assert_eq!(diff.text.description.removed.len(), 0);
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
}
