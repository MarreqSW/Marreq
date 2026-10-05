// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Attachments on requirements and verifications, and project storage (issue #241).
//!
//! Files are always served as downloads (`Content-Disposition: attachment`,
//! `nosniff`, a `sandbox` CSP), never rendered inline.

use std::io::Read;
use std::sync::Arc;

use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::http::{ContentType, Header};
use rocket::serde::{Deserialize, Serialize};

use crate::api::prelude::*;
use crate::auth::guards::{ProjectAccessOrBearer, ProjectBaselinesRead};
use crate::models::Attachment;
use crate::repository::{AttachmentsRepository, BaselineRepository, UserRepository};
use crate::services::attachment_service::{
    AttachmentEntity, AttachmentError, AttachmentService, NewUpload, format_mib, sanitize_filename,
};
use crate::storage::{AttachmentStorage, content_type};

/// The attachment storage: Rocket managed state, or the process-wide instance
/// (see [`crate::storage::install`]). 503 when neither is configured.
pub struct Storage(pub Arc<AttachmentStorage>);

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for Storage {
    type Error = ();

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let managed = request.rocket().state::<Arc<AttachmentStorage>>();
        match managed.or(crate::storage::installed()) {
            Some(storage) => rocket::request::Outcome::Success(Storage(Arc::clone(storage))),
            None => rocket::request::Outcome::Error((Status::ServiceUnavailable, ())),
        }
    }
}

impl From<AttachmentError> for ApiError {
    fn from(value: AttachmentError) -> Self {
        match value {
            AttachmentError::Repo(e) => e.into(),
            AttachmentError::BadInput(msg) => ApiError::BadRequest(msg),
            AttachmentError::TooLarge(msg) => ApiError::PayloadTooLarge(msg),
            AttachmentError::UnsupportedType(msg) => ApiError::UnsupportedMediaType(msg),
            AttachmentError::Storage(msg) => ApiError::Internal(msg),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct AttachmentView {
    pub id: i32,
    pub entity_type: String,
    pub entity_id: i32,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub uploaded_by: Option<i32>,
    /// Display name of the uploader, if the account still exists.
    pub uploaded_by_name: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    /// Only true in baseline listings, for files deleted after the baseline.
    pub deleted: bool,
}

fn views(state: &State<AppState>, rows: Vec<Attachment>) -> Vec<AttachmentView> {
    let repo = state.repo_read();
    let mut names = std::collections::HashMap::new();
    rows.into_iter()
        .map(|a| {
            let uploaded_by_name = a.uploaded_by.and_then(|uid| {
                names
                    .entry(uid)
                    .or_insert_with(|| repo.get_user_by_id(uid).ok().map(|u| u.name))
                    .clone()
            });
            AttachmentView {
                id: a.id,
                entity_type: a.entity_type,
                entity_id: a.entity_id,
                filename: a.original_filename,
                content_type: a.content_type,
                size_bytes: a.size_bytes,
                uploaded_by: a.uploaded_by,
                uploaded_by_name,
                created_at: a.created_at,
                deleted: a.deleted_at.is_some(),
            }
        })
        .collect()
}

fn parse_entity(entity_type: &str) -> ApiResult<AttachmentEntity> {
    AttachmentEntity::parse(entity_type).ok_or_else(|| {
        ApiError::BadRequest("entity_type must be 'requirement' or 'verification'".into())
    })
}

/// `GET /projects/<project_id>/attachments?entity_type=&entity_id=` — live attachments of one entity.
#[get("/projects/<project_id>/attachments?<entity_type>&<entity_id>")]
pub async fn list(
    access: ProjectAccessOrBearer,
    project_id: i32,
    entity_type: &str,
    entity_id: i32,
    state: &State<AppState>,
    storage: Storage,
) -> ApiResult<Json<Vec<AttachmentView>>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let entity = parse_entity(entity_type)?;
    let rows =
        AttachmentService::new(state.inner(), &storage.0).list(project_id, entity, entity_id)?;
    Ok(Json(views(state, rows)))
}

#[derive(FromForm)]
pub struct UploadForm<'r> {
    file: TempFile<'r>,
    entity_type: String,
    entity_id: i32,
}

/// First bytes of a file, for the type check.
fn read_head(path: &std::path::Path) -> std::io::Result<Vec<u8>> {
    let mut head = Vec::with_capacity(content_type::SNIFF_BYTES);
    std::fs::File::open(path)?
        .take(content_type::SNIFF_BYTES as u64)
        .read_to_end(&mut head)?;
    Ok(head)
}

/// `POST /projects/<project_id>/attachments` — multipart `file`, `entity_type`, `entity_id`.
///
/// 413 over the per-file limit or the project quota; 415 for a file type
/// outside the allowlist (checked against the file's bytes).
#[post("/projects/<project_id>/attachments", data = "<form>")]
pub async fn upload(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
    storage: Storage,
    mut form: Form<UploadForm<'_>>,
) -> ApiResult<(Status, Json<AttachmentView>)> {
    let user = access.user().clone();
    require_project_permission(state, &user, project_id, Permission::EditRequirements)?;
    let entity = parse_entity(&form.entity_type)?;
    let entity_id = form.entity_id;
    let storage: Arc<AttachmentStorage> = storage.0;
    let service = AttachmentService::new(state.inner(), &storage);
    service.ensure_entity(project_id, entity, entity_id)?;

    let filename = form
        .file
        .raw_name()
        .map(|n| n.dangerous_unsafe_unsanitized_raw().as_str().to_string())
        .and_then(|raw| sanitize_filename(&raw))
        .ok_or_else(|| ApiError::BadRequest("the upload needs a file name".into()))?;
    let size = form.file.len();
    if size == 0 {
        return Err(ApiError::BadRequest("the file is empty".into()));
    }
    if size > storage.config.max_file_bytes {
        return Err(ApiError::PayloadTooLarge(format!(
            "{filename} is {}; files can be at most {}",
            format_mib(size as i64),
            format_mib(storage.config.max_file_bytes as i64)
        )));
    }

    let tmp = storage
        .store()
        .new_upload_path()
        .map_err(|e| ApiError::Internal(format!("attachment storage is unavailable: {e}")))?;
    form.file
        .move_copy_to(&tmp)
        .await
        .map_err(|e| ApiError::Internal(format!("could not store the upload: {e}")))?;

    // Type check and hashing read the whole file: keep them off the async workers.
    let blocking_storage = Arc::clone(&storage);
    let checked_name = filename.clone();
    let staged = rocket::tokio::task::spawn_blocking(move || {
        let store = blocking_storage.store();
        let detected = read_head(&tmp)
            .map_err(|e| ApiError::Internal(e.to_string()))
            .and_then(|head| {
                content_type::detect(&checked_name, &head).map_err(ApiError::UnsupportedMediaType)
            });
        let kind = match detected {
            Ok(kind) => kind,
            Err(e) => {
                let _ = std::fs::remove_file(&tmp);
                return Err(e);
            }
        };
        store.stage(&tmp).map(|staged| (kind, staged)).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            ApiError::Internal(format!("could not read the upload: {e}"))
        })
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))??;
    let (kind, staged) = staged;

    let row = service.create(
        &user,
        NewUpload {
            project_id,
            entity,
            entity_id,
            filename,
            content_type: kind.content_type,
        },
        staged,
    )?;
    let view = views(state, vec![row]).remove(0);
    Ok((Status::Created, Json(view)))
}

/// A stored file streamed as a download.
#[derive(Responder)]
#[response(status = 200)]
pub struct AttachmentDownload {
    file: rocket::tokio::fs::File,
    content_type: ContentType,
    disposition: Header<'static>,
    nosniff: Header<'static>,
    csp: Header<'static>,
    cache: Header<'static>,
}

/// `attachment; filename="<ascii>"; filename*=UTF-8''<percent-encoded>` (RFC 6266 / 5987).
pub fn content_disposition(filename: &str) -> String {
    let ascii: String = filename
        .chars()
        .map(|c| {
            if c.is_ascii() && !c.is_ascii_control() && c != '"' && c != '\\' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!(
        "attachment; filename=\"{ascii}\"; filename*=UTF-8''{}",
        urlencoding::encode(filename)
    )
}

fn download_response(
    storage: &AttachmentStorage,
    row: &Attachment,
) -> ApiResult<AttachmentDownload> {
    let file = storage.store().open(&row.sha256).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ApiError::NotFound("the file is missing from storage".into())
        } else {
            ApiError::Internal(format!("could not read the file: {e}"))
        }
    })?;
    let content_type =
        ContentType::parse_flexible(&row.content_type).unwrap_or(ContentType::Binary);
    Ok(AttachmentDownload {
        file: rocket::tokio::fs::File::from_std(file),
        content_type,
        disposition: Header::new(
            "Content-Disposition",
            content_disposition(&row.original_filename),
        ),
        nosniff: Header::new("X-Content-Type-Options", "nosniff"),
        csp: Header::new("Content-Security-Policy", "default-src 'none'; sandbox"),
        cache: Header::new("Cache-Control", "private, no-cache"),
    })
}

/// `GET /projects/<project_id>/attachments/<attachment_id>/download`.
#[get("/projects/<project_id>/attachments/<attachment_id>/download")]
pub async fn download(
    access: ProjectAccessOrBearer,
    project_id: i32,
    attachment_id: i32,
    state: &State<AppState>,
    storage: Storage,
) -> ApiResult<AttachmentDownload> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let row =
        AttachmentService::new(state.inner(), &storage.0).get_live(project_id, attachment_id)?;
    download_response(&storage.0, &row)
}

/// `DELETE /projects/<project_id>/attachments/<attachment_id>`.
#[delete("/projects/<project_id>/attachments/<attachment_id>")]
pub async fn delete(
    access: ProjectAccessOrBearer,
    project_id: i32,
    attachment_id: i32,
    state: &State<AppState>,
    storage: Storage,
) -> ApiResult<Status> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;
    AttachmentService::new(state.inner(), &storage.0).delete(
        access.user(),
        project_id,
        attachment_id,
    )?;
    Ok(Status::NoContent)
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct StorageView {
    pub used_bytes: i64,
    pub quota_bytes: i64,
    pub quota_is_default: bool,
    pub default_quota_bytes: i64,
    pub retained_by_baselines_bytes: i64,
    pub max_file_bytes: i64,
    pub allowed_extensions: Vec<&'static str>,
}

fn storage_view(
    service: &AttachmentService<'_>,
    storage: &AttachmentStorage,
    project_id: i32,
) -> ApiResult<StorageView> {
    let usage = service.usage(project_id)?;
    let quota = service.quota(project_id)?;
    Ok(StorageView {
        used_bytes: usage.used_bytes,
        quota_bytes: quota.quota_bytes,
        quota_is_default: quota.is_default,
        default_quota_bytes: storage.config.default_project_quota_bytes as i64,
        retained_by_baselines_bytes: usage.retained_by_baselines_bytes,
        max_file_bytes: storage.config.max_file_bytes as i64,
        allowed_extensions: content_type::allowed_extensions(),
    })
}

/// `GET /projects/<project_id>/storage` — usage, quota and upload limits.
#[get("/projects/<project_id>/storage")]
pub async fn get_storage(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
    storage: Storage,
) -> ApiResult<Json<StorageView>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = AttachmentService::new(state.inner(), &storage.0);
    Ok(Json(storage_view(&service, &storage.0, project_id)?))
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", deny_unknown_fields)]
pub struct QuotaRequest {
    /// Megabytes, or `null` to go back to the instance default.
    pub quota_mb: Option<i64>,
}

const MAX_QUOTA_MB: i64 = 1024 * 1024 * 1024;

/// `PUT /projects/<project_id>/storage/quota` — instance administrators only.
///
/// A quota below current usage is allowed: it only blocks new uploads.
#[put("/projects/<project_id>/storage/quota", data = "<payload>")]
pub async fn set_quota(
    admin: AdminOnly,
    project_id: i32,
    payload: Json<QuotaRequest>,
    state: &State<AppState>,
    storage: Storage,
) -> ApiResult<Json<StorageView>> {
    use crate::repository::ProjectsRepository;
    state.repo_read().get_project_by_id(project_id)?;
    require_not_archived(state, project_id)?;
    let quota_bytes = match payload.quota_mb {
        None => None,
        Some(mb) if (1..=MAX_QUOTA_MB).contains(&mb) => Some(mb * 1024 * 1024),
        Some(_) => {
            return Err(ApiError::BadRequest(
                "quota_mb must be a whole number of megabytes of at least 1".into(),
            ));
        }
    };
    let service = AttachmentService::new(state.inner(), &storage.0);
    service.set_quota(&admin, project_id, quota_bytes)?;
    Ok(Json(storage_view(&service, &storage.0, project_id)?))
}

fn baseline_in_project(
    state: &State<AppState>,
    project_id: i32,
    baseline_id: i32,
) -> ApiResult<()> {
    let baseline = state.repo_read().get_baseline_by_id(baseline_id)?;
    if baseline.project_id != project_id {
        return Err(ApiError::NotFound(
            "baseline not found in this project".into(),
        ));
    }
    Ok(())
}

/// `GET /projects/<project_id>/baselines/<baseline_id>/attachments` — files recorded in a baseline.
#[get("/projects/<project_id>/baselines/<baseline_id>/attachments")]
pub async fn list_for_baseline(
    access: ProjectBaselinesRead,
    project_id: i32,
    baseline_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<AttachmentView>>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    baseline_in_project(state, project_id, baseline_id)?;
    let rows = state.repo_read().list_baseline_attachments(baseline_id)?;
    Ok(Json(views(state, rows)))
}

/// `GET /projects/<project_id>/baselines/<baseline_id>/attachments/<attachment_id>/download`
/// — also serves files deleted after the baseline was taken.
#[get("/projects/<project_id>/baselines/<baseline_id>/attachments/<attachment_id>/download")]
pub async fn download_for_baseline(
    access: ProjectBaselinesRead,
    project_id: i32,
    baseline_id: i32,
    attachment_id: i32,
    state: &State<AppState>,
    storage: Storage,
) -> ApiResult<AttachmentDownload> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    baseline_in_project(state, project_id, baseline_id)?;
    let row = state
        .repo_read()
        .list_baseline_attachments(baseline_id)?
        .into_iter()
        .find(|a| a.id == attachment_id)
        .ok_or_else(|| ApiError::NotFound("attachment not found in this baseline".into()))?;
    download_response(&storage.0, &row)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_disposition_has_ascii_fallback_and_utf8_name() {
        assert_eq!(
            content_disposition("report.pdf"),
            "attachment; filename=\"report.pdf\"; filename*=UTF-8''report.pdf"
        );
        assert_eq!(
            content_disposition("Größe \"v2\".pdf"),
            "attachment; filename=\"Gr__e _v2_.pdf\"; filename*=UTF-8''Gr%C3%B6%C3%9Fe%20%22v2%22.pdf"
        );
    }

    #[test]
    fn attachment_errors_map_to_statuses() {
        let status = |e: AttachmentError| ApiError::from(e).status();
        assert_eq!(
            status(AttachmentError::TooLarge("x".into())),
            Status::PayloadTooLarge
        );
        assert_eq!(
            status(AttachmentError::UnsupportedType("x".into())),
            Status::UnsupportedMediaType
        );
        assert_eq!(
            status(AttachmentError::BadInput("x".into())),
            Status::BadRequest
        );
        assert_eq!(
            status(AttachmentError::Repo(
                crate::repository::errors::RepoError::NotFound
            )),
            Status::NotFound
        );
    }
}
