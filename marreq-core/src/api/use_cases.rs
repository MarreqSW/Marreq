// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Use cases (issue #398, epic #388): list, create, read and delete; the
//! shared working draft (whole-document writes and operations); publishing;
//! versions and approval. See `docs/developer/use-cases-api.md`.

use std::collections::BTreeMap;

use rocket::Request;
use rocket::response::{Responder, Response};
use rocket::serde::{Deserialize, Serialize};

use crate::api::prelude::*;
use crate::auth::guards::{ProjectUseCasesApprove, ProjectUseCasesRead, ProjectUseCasesWrite};
use crate::models::{ActionType, EntityType, NewLog, UseCase, UseCaseDraft, UseCaseVersion, User};
use crate::repository::errors::RepoError;
use crate::repository::{LogRepository, UseCasesRepository, UserRepository};
use crate::services::use_case_service::{
    CreateUseCase, DraftChange, UseCaseError, UseCaseService, parse_document,
};
use crate::use_case_document::{
    BranchKind, Effect, Operation, OperationError, UseCaseDocument, labels,
};

/// An error with a JSON body: the standard `{status, error, message}` plus,
/// for some errors, a `code` and details.
#[derive(Debug)]
pub struct UseCaseApiError {
    status: Status,
    body: Value,
}

impl UseCaseApiError {
    fn new(status: Status, message: impl Into<String>, extra: Value) -> Self {
        let mut body = json!({
            "status": status.code,
            "error": status.reason_lossy(),
            "message": message.into(),
        });
        if let (Some(body), Value::Object(extra)) = (body.as_object_mut(), extra) {
            body.extend(extra);
        }
        Self { status, body }
    }
}

impl<'r> Responder<'r, 'static> for UseCaseApiError {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'static> {
        Response::build_from(self.body.respond_to(req)?)
            .status(self.status)
            .ok()
    }
}

impl From<ApiError> for UseCaseApiError {
    fn from(e: ApiError) -> Self {
        Self::new(e.status(), e.message(), Value::Null)
    }
}

impl From<RepoError> for UseCaseApiError {
    fn from(e: RepoError) -> Self {
        ApiError::from(e).into()
    }
}

fn draft_summary(state: &AppState, draft: &UseCaseDraft) -> DraftSummary {
    DraftSummary {
        revision: draft.revision,
        base_version_id: draft.base_version_id,
        updated_by: draft.updated_by,
        updated_by_name: user_name(state, draft.updated_by),
        updated_at: draft.updated_at,
    }
}

fn into_api_error(
    state: &AppState,
    doc: Option<&UseCaseDocument>,
    e: UseCaseError,
) -> UseCaseApiError {
    match e {
        UseCaseError::Repo(e) => e.into(),
        UseCaseError::NotFound(m) => UseCaseApiError::new(Status::NotFound, m, Value::Null),
        UseCaseError::BadRequest(m) => UseCaseApiError::new(Status::BadRequest, m, Value::Null),
        UseCaseError::Conflict(m) => UseCaseApiError::new(Status::Conflict, m, Value::Null),
        UseCaseError::InvalidDocument(errors) => UseCaseApiError::new(
            Status::UnprocessableEntity,
            format!(
                "invalid use case document: {}",
                errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            json!({ "code": "invalid_document", "errors": errors }),
        ),
        UseCaseError::Stale(current) => UseCaseApiError::new(
            Status::Conflict,
            "the draft changed since it was read; reload it and try again",
            json!({
                "code": "stale_revision",
                "current": current.map(|d| draft_summary(state, &d)),
            }),
        ),
        UseCaseError::Operation(op) => {
            let message = op.to_string();
            match op {
                OperationError::HasDependents {
                    index,
                    id,
                    branches,
                } => {
                    let labels = doc.map(labels).unwrap_or_default();
                    let branches: Vec<Value> = branches
                        .iter()
                        .map(|b| {
                            json!({
                                "id": b,
                                "label": labels.get(b),
                                "title": doc.and_then(|d| d.text_of(b)),
                            })
                        })
                        .collect();
                    UseCaseApiError::new(
                        Status::UnprocessableEntity,
                        message,
                        json!({
                            "code": "element_has_dependents",
                            "operation_index": index,
                            "element_id": id,
                            "element_label": labels.get(&id),
                            "branches": branches,
                        }),
                    )
                }
                OperationError::NotFound { index, id } => UseCaseApiError::new(
                    Status::UnprocessableEntity,
                    message,
                    json!({ "code": "unknown_element", "operation_index": index, "element_id": id }),
                ),
                OperationError::Invalid { index, .. } => UseCaseApiError::new(
                    Status::UnprocessableEntity,
                    message,
                    json!({ "code": "invalid_operation", "operation_index": index }),
                ),
            }
        }
    }
}

type UcResult<T> = Result<T, UseCaseApiError>;

fn user_name(state: &AppState, id: Option<i32>) -> Option<String> {
    id.and_then(|id| state.repo_read().get_user_by_id(id).ok())
        .map(|u| u.name)
}

fn require(
    state: &State<AppState>,
    user: &User,
    project_id: i32,
    permission: Permission,
) -> UcResult<()> {
    require_project_permission(state, user, project_id, permission).map_err(Into::into)
}

fn audit(
    state: &AppState,
    user: &User,
    action: ActionType,
    use_case: &UseCase,
    description: String,
    new_values: Option<Value>,
) {
    let _ = state.repo_write().insert_log(&NewLog {
        user_id: user.id,
        action_type: action.to_string(),
        entity_type: EntityType::UseCase.to_string(),
        entity_id: Some(use_case.id),
        project_id: Some(use_case.project_id),
        old_values: None,
        new_values: new_values.map(|v| v.to_string()),
        description: Some(description),
        ip_address: None,
        user_agent: None,
    });
}

// ----- Views -----

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct VersionSummary {
    pub id: i32,
    pub version_number: i32,
    pub approval_state: String,
    pub title: String,
    pub primary_actor: Option<String>,
    pub change_note: Option<String>,
    pub created_by: Option<i32>,
    pub created_by_name: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub reviewed_by: Option<i32>,
    pub reviewed_at: Option<chrono::NaiveDateTime>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct VersionView {
    #[serde(flatten)]
    pub summary: VersionSummary,
    pub document: Value,
    pub labels: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct DraftSummary {
    pub revision: i32,
    pub base_version_id: Option<i32>,
    pub updated_by: Option<i32>,
    pub updated_by_name: Option<String>,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct DraftView {
    pub use_case_id: i32,
    #[serde(flatten)]
    pub summary: DraftSummary,
    pub document: Value,
    pub labels: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct DraftChangeView {
    #[serde(flatten)]
    pub draft: DraftView,
    pub effects: Vec<Effect>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UseCaseDetail {
    pub id: i32,
    pub project_id: i32,
    pub reference_code: String,
    /// The current version's title, or the draft's before the first publish.
    pub title: String,
    pub created_by: Option<i32>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    /// The current version, or the one asked for with `?version_id=`.
    pub current_version: Option<VersionView>,
    pub draft: Option<DraftSummary>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct UseCaseRow {
    pub id: i32,
    pub reference_code: String,
    pub title: String,
    pub primary_actor: Option<String>,
    pub step_count: usize,
    pub alternative_count: usize,
    pub exception_count: usize,
    /// Of the current version; `null` before the first publish.
    pub approval_state: Option<String>,
    pub current_version_number: Option<i32>,
    pub has_draft: bool,
    pub updated_at: chrono::NaiveDateTime,
}

fn version_summary(state: &AppState, v: &UseCaseVersion) -> VersionSummary {
    VersionSummary {
        id: v.id,
        version_number: v.version_number,
        approval_state: v.approval_state.clone(),
        title: v.title.clone(),
        primary_actor: v.primary_actor.clone(),
        change_note: v.change_note.clone(),
        created_by: v.created_by,
        created_by_name: user_name(state, v.created_by),
        created_at: v.created_at,
        reviewed_by: v.reviewed_by,
        reviewed_at: v.reviewed_at,
        approved_by: v.approved_by,
        approved_at: v.approved_at,
    }
}

fn doc_labels(document: &Value) -> BTreeMap<String, String> {
    parse_document(document)
        .map(|d| labels(&d))
        .unwrap_or_default()
}

fn version_view(state: &AppState, v: &UseCaseVersion) -> VersionView {
    VersionView {
        summary: version_summary(state, v),
        document: v.document.clone(),
        labels: doc_labels(&v.document),
    }
}

fn draft_view(state: &AppState, d: &UseCaseDraft) -> DraftView {
    DraftView {
        use_case_id: d.use_case_id,
        summary: draft_summary(state, d),
        document: d.document.clone(),
        labels: doc_labels(&d.document),
    }
}

fn detail(
    state: &AppState,
    use_case: &UseCase,
    version: Option<UseCaseVersion>,
    draft: Option<UseCaseDraft>,
) -> UseCaseDetail {
    let title = version
        .as_ref()
        .map(|v| v.title.clone())
        .or_else(|| {
            draft
                .as_ref()
                .and_then(|d| parse_document(&d.document).ok())
                .map(|d| d.title)
        })
        .unwrap_or_default();
    UseCaseDetail {
        id: use_case.id,
        project_id: use_case.project_id,
        reference_code: use_case.reference_code.clone(),
        title,
        created_by: use_case.created_by,
        created_at: use_case.created_at,
        updated_at: use_case.updated_at,
        current_version: version.as_ref().map(|v| version_view(state, v)),
        draft: draft.as_ref().map(|d| draft_summary(state, d)),
    }
}

// ----- Use cases -----

/// GET /projects/{id}/use-cases — the project's use cases, filtered and sorted.
#[allow(clippy::too_many_arguments)]
#[get("/projects/<project_id>/use-cases?<q>&<approval_state>&<actor>&<sort_column>&<sort_dir>")]
pub async fn list(
    access: ProjectUseCasesRead,
    project_id: i32,
    q: Option<&str>,
    approval_state: Option<&str>,
    actor: Option<&str>,
    sort_column: Option<&str>,
    sort_dir: Option<&str>,
    state: &State<AppState>,
) -> UcResult<Json<Vec<UseCaseRow>>> {
    require(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let (use_cases, versions, drafts) = {
        let repo = state.repo_read();
        (
            repo.list_use_cases_by_project(project_id)?,
            repo.list_current_use_case_versions(project_id)?,
            repo.list_use_case_drafts_by_project(project_id)?,
        )
    };
    let mut rows: Vec<UseCaseRow> = use_cases
        .iter()
        .map(|u| {
            let version = versions.iter().find(|v| Some(v.id) == u.current_version_id);
            let draft = drafts.iter().find(|d| d.use_case_id == u.id);
            let doc = version
                .map(|v| &v.document)
                .or(draft.map(|d| &d.document))
                .and_then(|d| parse_document(d).ok());
            UseCaseRow {
                id: u.id,
                reference_code: u.reference_code.clone(),
                title: doc.as_ref().map(|d| d.title.clone()).unwrap_or_default(),
                primary_actor: doc.as_ref().and_then(|d| d.primary_actor.clone()),
                step_count: doc.as_ref().map_or(0, UseCaseDocument::step_count),
                alternative_count: doc
                    .as_ref()
                    .map_or(0, |d| d.branch_count(BranchKind::Alternative)),
                exception_count: doc
                    .as_ref()
                    .map_or(0, |d| d.branch_count(BranchKind::Exception)),
                approval_state: version.map(|v| v.approval_state.clone()),
                current_version_number: version.map(|v| v.version_number),
                has_draft: draft.is_some(),
                updated_at: u.updated_at,
            }
        })
        .collect();

    if let Some(q) = q.map(|q| q.trim().to_lowercase()).filter(|q| !q.is_empty()) {
        rows.retain(|r| {
            r.reference_code.to_lowercase().contains(&q)
                || r.title.to_lowercase().contains(&q)
                || r.primary_actor
                    .as_deref()
                    .is_some_and(|a| a.to_lowercase().contains(&q))
        });
    }
    if let Some(wanted) = approval_state.map(str::trim).filter(|s| !s.is_empty()) {
        rows.retain(|r| match wanted {
            "unpublished" => r.approval_state.is_none(),
            _ => r.approval_state.as_deref() == Some(wanted),
        });
    }
    if let Some(actor) = actor
        .map(|a| a.trim().to_lowercase())
        .filter(|a| !a.is_empty())
    {
        rows.retain(|r| r.primary_actor.as_deref().map(str::to_lowercase) == Some(actor.clone()));
    }
    match sort_column.unwrap_or("code") {
        "title" => rows.sort_by_key(|r| r.title.to_lowercase()),
        "actor" => rows.sort_by_key(|r| r.primary_actor.clone().unwrap_or_default().to_lowercase()),
        "approval" => rows.sort_by_key(|r| r.approval_state.clone().unwrap_or_default()),
        "modified" => rows.sort_by_key(|r| r.updated_at),
        _ => rows.sort_by(|a, b| a.reference_code.cmp(&b.reference_code)),
    }
    if sort_dir == Some("desc") {
        rows.reverse();
    }
    Ok(Json(rows))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct NextCode {
    pub reference_code: String,
}

/// GET /projects/{id}/use-cases/next-code — a suggestion for a new use case.
#[get("/projects/<project_id>/use-cases/next-code")]
pub async fn next_code(
    access: ProjectUseCasesRead,
    project_id: i32,
    state: &State<AppState>,
) -> UcResult<Json<NextCode>> {
    require(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let reference_code = UseCaseService::new(state.inner())
        .next_code(project_id)
        .map_err(|e| into_api_error(state, None, e))?;
    Ok(Json(NextCode { reference_code }))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct CreateUseCaseRequest {
    #[serde(default)]
    pub reference_code: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub primary_actor: Option<String>,
    /// A whole `marreq.use-case.v1` document to start the draft from.
    #[serde(default)]
    pub document: Option<Value>,
}

/// POST /projects/{id}/use-cases — create a use case and its draft.
#[post("/projects/<project_id>/use-cases", data = "<body>")]
pub async fn create(
    access: ProjectUseCasesWrite,
    project_id: i32,
    body: Json<CreateUseCaseRequest>,
    state: &State<AppState>,
) -> UcResult<(Status, Json<UseCaseDetail>)> {
    let user = access.user();
    require(state, user, project_id, Permission::EditRequirements)?;
    let body = body.into_inner();
    let (use_case, draft) = UseCaseService::new(state.inner())
        .create(
            user,
            project_id,
            CreateUseCase {
                reference_code: body.reference_code,
                title: body.title,
                primary_actor: body.primary_actor,
                document: body.document,
            },
        )
        .map_err(|e| into_api_error(state, None, e))?;
    audit(
        state,
        user,
        ActionType::Create,
        &use_case,
        format!("Created use case {}", use_case.reference_code),
        None,
    );
    Ok((
        Status::Created,
        Json(detail(state, &use_case, None, Some(draft))),
    ))
}

/// GET /projects/{id}/use-cases/{uc} — the use case with its current version
/// (or `?version_id=`) and a summary of its draft.
#[get("/projects/<project_id>/use-cases/<id>?<version_id>")]
pub async fn get(
    access: ProjectUseCasesRead,
    project_id: i32,
    id: i32,
    version_id: Option<i32>,
    state: &State<AppState>,
) -> UcResult<Json<UseCaseDetail>> {
    require(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = UseCaseService::new(state.inner());
    let err = |e| into_api_error(state, None, e);
    let use_case = service.get_in_project(project_id, id).map_err(err)?;
    let version = match version_id {
        Some(vid) => Some(service.get_version(&use_case, vid).map_err(err)?),
        None => service.current_version(&use_case).map_err(err)?,
    };
    let draft = service.draft(&use_case).map_err(err)?;
    Ok(Json(detail(state, &use_case, version, draft)))
}

/// DELETE /projects/{id}/use-cases/{uc} — delete the use case, its versions
/// and its draft.
#[delete("/projects/<project_id>/use-cases/<id>")]
pub async fn delete(
    access: ProjectUseCasesWrite,
    project_id: i32,
    id: i32,
    state: &State<AppState>,
) -> UcResult<Status> {
    let user = access.user();
    require(state, user, project_id, Permission::EditRequirements)?;
    let service = UseCaseService::new(state.inner());
    let err = |e| into_api_error(state, None, e);
    let use_case = service.get_in_project(project_id, id).map_err(err)?;
    service.delete(&use_case).map_err(err)?;
    audit(
        state,
        user,
        ActionType::Delete,
        &use_case,
        format!("Deleted use case {}", use_case.reference_code),
        None,
    );
    Ok(Status::NoContent)
}

// ----- Draft -----

/// GET /projects/{id}/use-cases/{uc}/draft — 404 when there is no draft.
#[get("/projects/<project_id>/use-cases/<id>/draft")]
pub async fn get_draft(
    access: ProjectUseCasesRead,
    project_id: i32,
    id: i32,
    state: &State<AppState>,
) -> UcResult<Json<DraftView>> {
    require(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = UseCaseService::new(state.inner());
    let err = |e| into_api_error(state, None, e);
    let use_case = service.get_in_project(project_id, id).map_err(err)?;
    match service.draft(&use_case).map_err(err)? {
        Some(draft) => Ok(Json(draft_view(state, &draft))),
        None => Err(UseCaseApiError::new(
            Status::NotFound,
            "there is no draft",
            Value::Null,
        )),
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct PutDraftRequest {
    /// The draft revision this write is based on; `null` when there is no
    /// draft yet.
    pub base_revision: Option<i32>,
    pub document: Value,
}

/// PUT /projects/{id}/use-cases/{uc}/draft — replace the whole draft.
#[put("/projects/<project_id>/use-cases/<id>/draft", data = "<body>")]
pub async fn put_draft(
    access: ProjectUseCasesWrite,
    project_id: i32,
    id: i32,
    body: Json<PutDraftRequest>,
    state: &State<AppState>,
) -> UcResult<Json<DraftChangeView>> {
    let body = body.into_inner();
    change_draft(
        state,
        access.user(),
        project_id,
        id,
        body.base_revision,
        DraftChange::Replace(body.document),
    )
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct OperationsRequest {
    pub base_revision: Option<i32>,
    pub operations: Vec<Operation>,
}

/// POST /projects/{id}/use-cases/{uc}/draft/operations — apply a batch of
/// operations atomically. The first write opens a draft from the current
/// version.
#[post(
    "/projects/<project_id>/use-cases/<id>/draft/operations",
    data = "<body>"
)]
pub async fn draft_operations(
    access: ProjectUseCasesWrite,
    project_id: i32,
    id: i32,
    body: Json<OperationsRequest>,
    state: &State<AppState>,
) -> UcResult<Json<DraftChangeView>> {
    let body = body.into_inner();
    if body.operations.is_empty() {
        return Err(UseCaseApiError::new(
            Status::BadRequest,
            "operations must not be empty",
            Value::Null,
        ));
    }
    change_draft(
        state,
        access.user(),
        project_id,
        id,
        body.base_revision,
        DraftChange::Operations(body.operations),
    )
}

fn change_draft(
    state: &State<AppState>,
    user: &User,
    project_id: i32,
    id: i32,
    base_revision: Option<i32>,
    change: DraftChange,
) -> UcResult<Json<DraftChangeView>> {
    require(state, user, project_id, Permission::EditRequirements)?;
    let service = UseCaseService::new(state.inner());
    let use_case = service
        .get_in_project(project_id, id)
        .map_err(|e| into_api_error(state, None, e))?;
    // The document the operations were applied to, to describe dependents.
    let before = service
        .draft(&use_case)
        .ok()
        .flatten()
        .map(|d| d.document)
        .or_else(|| {
            service
                .current_version(&use_case)
                .ok()
                .flatten()
                .map(|v| v.document)
        })
        .and_then(|d| parse_document(&d).ok());
    let (draft, effects) = service
        .change_draft(user, &use_case, base_revision, change)
        .map_err(|e| into_api_error(state, before.as_ref(), e))?;
    Ok(Json(DraftChangeView {
        draft: draft_view(state, &draft),
        effects,
    }))
}

/// DELETE /projects/{id}/use-cases/{uc}/draft — discard the draft.
#[delete("/projects/<project_id>/use-cases/<id>/draft")]
pub async fn discard_draft(
    access: ProjectUseCasesWrite,
    project_id: i32,
    id: i32,
    state: &State<AppState>,
) -> UcResult<Status> {
    require(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;
    let service = UseCaseService::new(state.inner());
    let err = |e| into_api_error(state, None, e);
    let use_case = service.get_in_project(project_id, id).map_err(err)?;
    service.discard_draft(&use_case).map_err(err)?;
    Ok(Status::NoContent)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct PublishRequest {
    pub base_revision: i32,
    #[serde(default)]
    pub change_note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct PublishResponse {
    pub version: VersionView,
}

/// POST /projects/{id}/use-cases/{uc}/draft/publish — publish the draft as
/// the next version, in the `draft` approval state.
#[post("/projects/<project_id>/use-cases/<id>/draft/publish", data = "<body>")]
pub async fn publish(
    access: ProjectUseCasesWrite,
    project_id: i32,
    id: i32,
    body: Json<PublishRequest>,
    state: &State<AppState>,
) -> UcResult<(Status, Json<PublishResponse>)> {
    let user = access.user();
    require(state, user, project_id, Permission::EditRequirements)?;
    let body = body.into_inner();
    let service = UseCaseService::new(state.inner());
    let use_case = service
        .get_in_project(project_id, id)
        .map_err(|e| into_api_error(state, None, e))?;
    let version = service
        .publish(user, &use_case, body.base_revision, body.change_note)
        .map_err(|e| into_api_error(state, None, e))?;
    audit(
        state,
        user,
        ActionType::Update,
        &use_case,
        format!(
            "Published version {} of use case {}",
            version.version_number, use_case.reference_code
        ),
        Some(json!({ "version_id": version.id, "version_number": version.version_number })),
    );
    Ok((
        Status::Created,
        Json(PublishResponse {
            version: version_view(state, &version),
        }),
    ))
}

// ----- Versions -----

/// GET /projects/{id}/use-cases/{uc}/versions — newest first, without documents.
#[get("/projects/<project_id>/use-cases/<id>/versions")]
pub async fn list_versions(
    access: ProjectUseCasesRead,
    project_id: i32,
    id: i32,
    state: &State<AppState>,
) -> UcResult<Json<Vec<VersionSummary>>> {
    require(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = UseCaseService::new(state.inner());
    let use_case = service
        .get_in_project(project_id, id)
        .map_err(|e| into_api_error(state, None, e))?;
    let versions = state.repo_read().list_use_case_versions(use_case.id)?;
    Ok(Json(
        versions.iter().map(|v| version_summary(state, v)).collect(),
    ))
}

/// GET /projects/{id}/use-cases/{uc}/versions/{v} — one version with its
/// document and labels.
#[get("/projects/<project_id>/use-cases/<id>/versions/<version_id>")]
pub async fn get_version(
    access: ProjectUseCasesRead,
    project_id: i32,
    id: i32,
    version_id: i32,
    state: &State<AppState>,
) -> UcResult<Json<VersionView>> {
    require(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = UseCaseService::new(state.inner());
    let err = |e| into_api_error(state, None, e);
    let use_case = service.get_in_project(project_id, id).map_err(err)?;
    let version = service.get_version(&use_case, version_id).map_err(err)?;
    Ok(Json(version_view(state, &version)))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct SetApprovalRequest {
    pub state: String,
}

/// PUT /projects/{id}/use-cases/{uc}/versions/{v}/approval — `reviewed` or
/// `approved`, by a project reviewer, as for requirement versions.
#[put(
    "/projects/<project_id>/use-cases/<id>/versions/<version_id>/approval",
    data = "<body>"
)]
pub async fn set_version_approval(
    access: ProjectUseCasesApprove,
    project_id: i32,
    id: i32,
    version_id: i32,
    body: Json<SetApprovalRequest>,
    state: &State<AppState>,
) -> UcResult<Json<VersionView>> {
    let user = access.user();
    require_not_archived(state, project_id)?;
    let service = UseCaseService::new(state.inner());
    let err = |e| into_api_error(state, None, e);
    let use_case = service.get_in_project(project_id, id).map_err(err)?;
    let version = service.get_version(&use_case, version_id).map_err(err)?;
    require_project_reviewer(state, user, project_id)?;
    let updated = service
        .set_approval(user, &version, &body.state)
        .map_err(err)?;
    if updated.approval_state != version.approval_state {
        audit(
            state,
            user,
            ActionType::StatusChange,
            &use_case,
            format!(
                "Use case {} version {}: {} → {}",
                use_case.reference_code,
                updated.version_number,
                version.approval_state,
                updated.approval_state
            ),
            Some(json!({ "version_id": updated.id, "approval_state": updated.approval_state })),
        );
    }
    Ok(Json(version_view(state, &updated)))
}
