// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Attachments on requirements and verifications, with a per-project storage
//! quota (issue #241).
//!
//! Upload order: the file is hashed in the store's scratch area, the row is
//! inserted under the quota check, and only then is the file moved into
//! place. A rejected upload never touches stored files.

use std::sync::Arc;

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{Attachment, NewAttachment, User};
use crate::repository::errors::RepoError;
use crate::repository::{
    AttachmentsRepository, QuotaCheck, RequirementsRepository, StorageUsage,
    VerificationsRepository,
};
use crate::services::AuditLog;
use crate::storage::{AttachmentStorage, StagedBlob};

/// What an attachment belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentEntity {
    Requirement,
    Verification,
}

impl AttachmentEntity {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "requirement" => Some(Self::Requirement),
            "verification" => Some(Self::Verification),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Requirement => "requirement",
            Self::Verification => "verification",
        }
    }
}

#[derive(Debug)]
pub enum AttachmentError {
    Repo(RepoError),
    BadInput(String),
    /// Over the per-file limit or the project quota.
    TooLarge(String),
    UnsupportedType(String),
    Storage(String),
}

impl From<RepoError> for AttachmentError {
    fn from(value: RepoError) -> Self {
        Self::Repo(value)
    }
}

/// Effective quota for a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectQuota {
    pub quota_bytes: i64,
    /// True when no per-project override is set.
    pub is_default: bool,
}

/// Where a checked upload goes and what it is.
pub struct NewUpload {
    pub project_id: i32,
    pub entity: AttachmentEntity,
    pub entity_id: i32,
    /// Already sanitised with [`sanitize_filename`].
    pub filename: String,
    pub content_type: &'static str,
}

const MAX_FILENAME_CHARS: usize = 255;

/// Keep only the final path component, drop control characters and cap the
/// length (shortening the stem so the extension survives).
pub fn sanitize_filename(raw: &str) -> Option<String> {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or("");
    let cleaned: String = base.chars().filter(|c| !c.is_control()).collect();
    let cleaned = cleaned.trim().trim_matches('.').trim().to_string();
    if cleaned.is_empty() {
        return None;
    }
    if cleaned.chars().count() <= MAX_FILENAME_CHARS {
        return Some(cleaned);
    }
    let (stem, ext) = match cleaned.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && ext.chars().count() <= 16 => {
            (stem.to_string(), format!(".{ext}"))
        }
        _ => (cleaned.clone(), String::new()),
    };
    let keep = MAX_FILENAME_CHARS - ext.chars().count();
    Some(stem.chars().take(keep).collect::<String>() + &ext)
}

/// Human-readable size for error messages: two decimals below 10 MB so
/// "used + needed > quota" still reads correctly after rounding.
pub fn format_mib(bytes: i64) -> String {
    let mib = bytes as f64 / (1024.0 * 1024.0);
    if mib < 10.0 {
        format!("{mib:.2} MB")
    } else {
        format!("{mib:.1} MB")
    }
}

pub struct AttachmentService<'a> {
    state: &'a AppState<DieselCachedRepo>,
    storage: &'a Arc<AttachmentStorage>,
}

impl<'a> AttachmentService<'a> {
    pub fn new(state: &'a AppState<DieselCachedRepo>, storage: &'a Arc<AttachmentStorage>) -> Self {
        Self { state, storage }
    }

    /// 404 unless the requirement or verification exists in this project.
    pub fn ensure_entity(
        &self,
        project_id: i32,
        entity: AttachmentEntity,
        entity_id: i32,
    ) -> Result<(), AttachmentError> {
        let repo = self.state.repo_read();
        let owner = match entity {
            AttachmentEntity::Requirement => {
                repo.get_requirement_by_id(entity_id).map(|r| r.project_id)
            }
            AttachmentEntity::Verification => {
                repo.get_verification_by_id(entity_id).map(|v| v.project_id)
            }
        };
        match owner {
            Ok(pid) if pid == project_id => Ok(()),
            Ok(_)
            | Err(RepoError::NotFound)
            | Err(RepoError::Db(diesel::result::Error::NotFound)) => {
                Err(RepoError::NotFound.into())
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn list(
        &self,
        project_id: i32,
        entity: AttachmentEntity,
        entity_id: i32,
    ) -> Result<Vec<Attachment>, AttachmentError> {
        self.ensure_entity(project_id, entity, entity_id)?;
        Ok(self
            .state
            .repo_read()
            .list_attachments(project_id, entity.as_str(), entity_id)?)
    }

    /// A live attachment of this project.
    pub fn get_live(&self, project_id: i32, id: i32) -> Result<Attachment, AttachmentError> {
        let row = self.state.repo_read().get_attachment(id)?;
        if row.project_id != project_id || row.deleted_at.is_some() {
            return Err(RepoError::NotFound.into());
        }
        Ok(row)
    }

    pub fn quota(&self, project_id: i32) -> Result<ProjectQuota, AttachmentError> {
        let override_bytes = self
            .state
            .repo_read()
            .get_project_storage_quota(project_id)?;
        Ok(match override_bytes {
            Some(quota_bytes) => ProjectQuota {
                quota_bytes,
                is_default: false,
            },
            None => ProjectQuota {
                quota_bytes: self.storage.config.default_project_quota_bytes as i64,
                is_default: true,
            },
        })
    }

    /// Set or clear (`None`) a project's quota override. Instance admins only
    /// (enforced by the route). Recorded in the audit log.
    pub fn set_quota(
        &self,
        actor: &User,
        project_id: i32,
        quota_bytes: Option<i64>,
    ) -> Result<ProjectQuota, AttachmentError> {
        let before = self.quota(project_id)?;
        self.state
            .repo_write()
            .set_project_storage_quota(project_id, quota_bytes, actor.id)?;
        let after = self.quota(project_id)?;
        if let Ok(mut conn) = self.audit_conn() {
            let describe = |q: ProjectQuota| {
                if q.is_default {
                    format!("default ({})", format_mib(q.quota_bytes))
                } else {
                    format_mib(q.quota_bytes)
                }
            };
            let _ = crate::logger::Logger::log_custom(
                conn.as_mut(),
                &crate::logger::LogCtx::new(actor.id),
                crate::models::ActionType::Update,
                crate::models::EntityType::Project,
                Some(project_id),
                Some(project_id),
                None,
                None,
                Some(format!(
                    "Storage quota changed from {} to {}",
                    describe(before),
                    describe(after)
                )),
            );
        }
        Ok(after)
    }

    pub fn usage(&self, project_id: i32) -> Result<StorageUsage, AttachmentError> {
        Ok(self.state.repo_read().project_storage_usage(project_id)?)
    }

    /// Record a staged upload and move it into the store. On any failure the
    /// staged file is removed.
    pub fn create(
        &self,
        actor: &User,
        upload: NewUpload,
        staged: StagedBlob,
    ) -> Result<Attachment, AttachmentError> {
        let NewUpload {
            project_id,
            entity,
            entity_id,
            filename,
            content_type,
        } = upload;
        let store = self.storage.store();
        let quota = match self.quota(project_id) {
            Ok(q) => q,
            Err(e) => {
                store.discard(staged);
                return Err(e);
            }
        };
        let new = NewAttachment {
            project_id,
            entity_type: entity.as_str().to_string(),
            entity_id,
            sha256: staged.sha256.clone(),
            size_bytes: staged.size as i64,
            original_filename: filename,
            content_type: content_type.to_string(),
            uploaded_by: Some(actor.id),
        };
        let outcome = self
            .state
            .repo_write()
            .create_attachment_within_quota(&new, quota.quota_bytes);
        let row = match outcome {
            Ok(QuotaCheck::Created(row)) => row,
            Ok(QuotaCheck::Exceeded { used_bytes }) => {
                store.discard(staged);
                return Err(AttachmentError::TooLarge(format!(
                    "Project storage limit reached: {} of {} used, and this file needs {}",
                    format_mib(used_bytes),
                    format_mib(quota.quota_bytes),
                    format_mib(new.size_bytes)
                )));
            }
            Err(e) => {
                store.discard(staged);
                return Err(e.into());
            }
        };
        if let Err(e) = self.storage.commit(staged) {
            // Keep the database consistent with the store.
            let _ = self.remove_row(&row);
            return Err(AttachmentError::Storage(format!(
                "could not store the file: {e}"
            )));
        }
        self.audit_created(actor, row.id, &row);
        Ok(row)
    }

    /// Soft-delete an attachment; the row and file go away unless a baseline keeps them.
    pub fn delete(
        &self,
        actor: &User,
        project_id: i32,
        id: i32,
    ) -> Result<Attachment, AttachmentError> {
        self.get_live(project_id, id)?;
        let deleted = self.state.repo_write().soft_delete_attachment(id)?;
        self.purge(&deleted);
        self.audit_deleted(actor, &deleted);
        Ok(deleted)
    }

    fn remove_row(&self, row: &Attachment) -> Result<(), AttachmentError> {
        let mut repo = self.state.repo_write();
        repo.soft_delete_attachment(row.id)?;
        repo.purge_attachment_if_unreferenced(row.id)?;
        Ok(())
    }

    fn purge(&self, row: &Attachment) {
        purge_row(self.state, Some(self.storage), row.id);
    }
}

/// Hard-delete a soft-deleted row unless a baseline keeps it, and remove the
/// file once nothing references it. Best effort: a failure leaves an
/// unreferenced file, never a row without its file.
fn purge_row(
    state: &AppState<DieselCachedRepo>,
    storage: Option<&Arc<AttachmentStorage>>,
    id: i32,
) {
    let unused_sha = state.repo_write().purge_attachment_if_unreferenced(id);
    if let (Ok(Some(sha)), Some(storage)) = (unused_sha, storage) {
        let _ = storage.delete_if_unused(&sha, || {
            state
                .repo_read()
                .attachment_blob_in_use(&sha)
                .unwrap_or(true)
        });
    }
}

/// Soft-delete the attachments of a requirement or verification that is being
/// deleted. Files are removed through the process-wide storage when installed.
pub fn on_entity_deleted(
    state: &AppState<DieselCachedRepo>,
    project_id: i32,
    entity: AttachmentEntity,
    entity_id: i32,
) -> Result<(), RepoError> {
    let deleted = state.repo_write().soft_delete_attachments_for_entity(
        project_id,
        entity.as_str(),
        entity_id,
    )?;
    let storage = crate::storage::installed();
    for row in &deleted {
        purge_row(state, storage, row.id);
    }
    Ok(())
}

impl AuditLog for AttachmentService<'_> {
    fn app_state(&self) -> &AppState<DieselCachedRepo> {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_keeps_the_last_path_component() {
        assert_eq!(
            sanitize_filename("report.pdf").as_deref(),
            Some("report.pdf")
        );
        assert_eq!(
            sanitize_filename("C:\\Users\\me\\Desktop\\plan v2.docx").as_deref(),
            Some("plan v2.docx")
        );
        assert_eq!(
            sanitize_filename("../../etc/passwd").as_deref(),
            Some("passwd")
        );
        assert_eq!(
            sanitize_filename("  spaced.txt  ").as_deref(),
            Some("spaced.txt")
        );
    }

    #[test]
    fn sanitize_drops_control_characters_and_empty_names() {
        assert_eq!(
            sanitize_filename("a\u{0}b\nc.txt").as_deref(),
            Some("abc.txt")
        );
        assert_eq!(sanitize_filename("dir/"), None);
        assert_eq!(sanitize_filename(".."), None);
        assert_eq!(sanitize_filename("   "), None);
    }

    #[test]
    fn sanitize_caps_length_but_keeps_the_extension() {
        let long = format!("{}.pdf", "é".repeat(300));
        let out = sanitize_filename(&long).unwrap();
        assert_eq!(out.chars().count(), 255);
        assert!(out.ends_with(".pdf"));
    }

    #[test]
    fn entity_names_round_trip() {
        for e in [
            AttachmentEntity::Requirement,
            AttachmentEntity::Verification,
        ] {
            assert_eq!(AttachmentEntity::parse(e.as_str()), Some(e));
        }
        assert_eq!(AttachmentEntity::parse("project"), None);
    }

    #[test]
    fn formats_sizes_in_megabytes() {
        assert_eq!(format_mib(500 * 1024 * 1024), "500.0 MB");
        assert_eq!(format_mib(1536 * 1024), "1.50 MB");
        assert_eq!(format_mib(1_200_000), "1.14 MB");
    }
}
