// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

//! Document builder for creating embedding source text.
//!
//! Constructs a deterministic text representation of a requirement for embedding,
//! including all relevant fields and metadata.

use crate::models::DecoratedRequirement;
use sha2::{Digest, Sha256};

/// Build the embedding document text for a requirement.
///
/// Creates a structured text blob that includes all searchable fields:
/// - Reference code (weighted highest)
/// - Title (weighted high)
/// - Description (main content)
/// - Justification/rationale
/// - Category, applicability, verification, status metadata
/// - Parent chain information
///
/// The format is designed to be:
/// 1. Deterministic (same requirement always produces same text)
/// 2. Stable (field ordering is fixed)
/// 3. Comprehensive (includes all relevant searchable content)
pub fn build_embedding_document(req: &DecoratedRequirement) -> String {
    let mut parts = Vec::new();

    // Reference code (highest weight in search)
    if !req.reference_code.is_empty() {
        parts.push(format!("[REF] {}", req.reference_code));
    }

    // Title (high weight)
    if !req.title.is_empty() {
        parts.push(format!("[TITLE] {}", req.title));
    }

    // Description (main content)
    if !req.description.is_empty() {
        parts.push(format!("[DESC] {}", req.description));
    }

    // Justification/rationale
    if let Some(ref justification) = req.justification {
        if !justification.is_empty() {
            parts.push(format!("[RATIONALE] {}", justification));
        }
    }

    // Category metadata
    if !req.category_id.is_empty() {
        parts.push(format!("[CATEGORY] {}", req.category_id));
    }

    // Applicability metadata
    if !req.applicability_id.is_empty() {
        parts.push(format!("[APPLICABILITY] {}", req.applicability_id));
    }

    // Verification method
    if !req.verification_method_id.is_empty() {
        parts.push(format!("[VERIFICATION] {}", req.verification_method_id));
    }

    // Status
    if !req.status_id.is_empty() {
        parts.push(format!("[STATUS] {}", req.status_id));
    }

    // Parent chain
    if !req.req_parent_title.is_empty() {
        parts.push(format!("[PARENT] {}", req.req_parent_title));
    }

    parts.join("\n")
}

/// Compute a content hash for change detection.
///
/// Uses SHA-256 to hash the embedding document text combined with the model ID.
/// This allows detecting when a requirement's content has changed and needs
/// re-embedding, or when the model has changed.
pub fn compute_content_hash(document: &str, model_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(document.as_bytes());
    hasher.update(b"|");
    hasher.update(model_id.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// Check if a requirement needs re-indexing.
pub fn needs_reindex(
    req: &DecoratedRequirement,
    current_hash: Option<&str>,
    model_id: &str,
) -> bool {
    let document = build_embedding_document(req);
    let new_hash = compute_content_hash(&document, model_id);

    match current_hash {
        Some(hash) => hash != new_hash,
        None => true,
    }
}

// Add hex encoding since we use it
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_requirement() -> DecoratedRequirement {
        DecoratedRequirement {
            id: 1,
            current_version_id: None,
            title: "System shall process inputs".into(),
            description: "The system shall process all valid inputs within 100ms".into(),
            verification_method_id: "Analysis".into(),
            req_verification_ids: vec![1],
            status_id: "Draft".into(),
            req_current_status_id: 1,
            status_tag_color: None,
            author_id: "John Doe".into(),
            req_author_id: 1,
            reviewer_id: "Jane Doe".into(),
            req_reviewer_id: 2,
            reference_code: "REQ-SYS-001".into(),
            category_id: "Functional".into(),
            req_category_id: 1,
            applicability_id: "All Variants".into(),
            req_applicability_id: 1,
            req_parent_id: Some(0),
            req_parent_title: "System Requirements".into(),
            req_parents: vec![],
            req_parent_reference_code: "".into(),
            req_parent_description: "".into(),
            req_parent_status_id: "".into(),
            req_parent_status_tag_color: None,
            req_parent_category_id: "".into(),
            creation_date: "2024-01-01".into(),
            update_date: "2024-01-15".into(),
            deadline_date: "".into(),
            justification: Some("Required for real-time operation".into()),
            project_id: 1,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        }
    }

    #[test]
    fn build_document_includes_all_fields() {
        let req = sample_requirement();
        let doc = build_embedding_document(&req);

        assert!(doc.contains("[REF] REQ-SYS-001"));
        assert!(doc.contains("[TITLE] System shall process inputs"));
        assert!(doc.contains("[DESC] The system shall process all valid inputs"));
        assert!(doc.contains("[RATIONALE] Required for real-time operation"));
        assert!(doc.contains("[CATEGORY] Functional"));
        assert!(doc.contains("[APPLICABILITY] All Variants"));
        assert!(doc.contains("[VERIFICATION] Analysis"));
        assert!(doc.contains("[STATUS] Draft"));
        assert!(doc.contains("[PARENT] System Requirements"));
    }

    #[test]
    fn build_document_deterministic() {
        let req = sample_requirement();
        let doc1 = build_embedding_document(&req);
        let doc2 = build_embedding_document(&req);

        assert_eq!(
            doc1, doc2,
            "Same requirement should produce identical document"
        );
    }

    #[test]
    fn build_document_handles_missing_justification() {
        let mut req = sample_requirement();
        req.justification = None;

        let doc = build_embedding_document(&req);
        assert!(!doc.contains("[RATIONALE]"));
    }

    #[test]
    fn build_document_handles_empty_fields() {
        let mut req = sample_requirement();
        req.reference_code = "".into();
        req.req_parent_title = "".into();

        let doc = build_embedding_document(&req);
        assert!(!doc.contains("[REF]"));
        assert!(!doc.contains("[PARENT]"));
    }

    #[test]
    fn content_hash_deterministic() {
        let doc = "Test document content";
        let model = "text-embedding-3-small";

        let hash1 = compute_content_hash(doc, model);
        let hash2 = compute_content_hash(doc, model);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn content_hash_changes_with_model() {
        let doc = "Test document content";

        let hash1 = compute_content_hash(doc, "model-a");
        let hash2 = compute_content_hash(doc, "model-b");

        assert_ne!(
            hash1, hash2,
            "Different models should produce different hashes"
        );
    }

    #[test]
    fn content_hash_changes_with_content() {
        let model = "text-embedding-3-small";

        let hash1 = compute_content_hash("Document A", model);
        let hash2 = compute_content_hash("Document B", model);

        assert_ne!(
            hash1, hash2,
            "Different content should produce different hashes"
        );
    }

    #[test]
    fn needs_reindex_true_when_no_hash() {
        let req = sample_requirement();
        assert!(needs_reindex(&req, None, "model"));
    }

    #[test]
    fn needs_reindex_true_when_hash_differs() {
        let req = sample_requirement();
        assert!(needs_reindex(&req, Some("old-hash"), "model"));
    }

    #[test]
    fn needs_reindex_false_when_hash_matches() {
        let req = sample_requirement();
        let doc = build_embedding_document(&req);
        let hash = compute_content_hash(&doc, "model");

        assert!(!needs_reindex(&req, Some(&hash), "model"));
    }

    #[test]
    fn needs_reindex_true_when_model_changes() {
        let req = sample_requirement();
        let doc = build_embedding_document(&req);
        let hash = compute_content_hash(&doc, "old-model");

        assert!(needs_reindex(&req, Some(&hash), "new-model"));
    }
}
