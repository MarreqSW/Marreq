// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Report documents (issue #354): the catalog of report types and sections,
//! saved report templates, and PDF / ODT generation.

use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

use crate::api::exports::FileDownload;
use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::models::{ActionType, EntityType, NewLog, ReportTemplate, ReportTemplateWrite, User};
use crate::permissions::ROLE_ADMIN;
use crate::reports::definition::{DefaultContext, ReportDefinition, ReportType};
use crate::reports::sections::{SectionSpec, catalog};
use crate::reports::{ReportError, ReportFormat};
use crate::repository::{
    LogRepository, ProjectMembersRepository, ProjectsRepository, ReportTemplateRepository,
    UserRepository,
};

const NAME_MAX: usize = 120;

fn default_context(state: &AppState, user: &User, project_id: i32) -> ApiResult<DefaultContext> {
    let project = state
        .repo_read()
        .get_project_by_id(project_id)
        .map_err(ApiError::from)?;
    Ok(DefaultContext {
        project_name: project.name,
        project_slug: project.slug,
        user_name: if user.name.trim().is_empty() {
            user.username.clone()
        } else {
            user.name.clone()
        },
        today: crate::reports::today(),
    })
}

/// The built-in template of `report_type` for this project and user.
pub fn default_definition(
    state: &AppState,
    user: &User,
    project_id: i32,
    report_type: ReportType,
) -> ApiResult<ReportDefinition> {
    Ok(ReportDefinition::default_for(
        report_type,
        &default_context(state, user, project_id)?,
    ))
}

pub fn report_error(e: ReportError) -> ApiError {
    match e {
        ReportError::Repo(e) => ApiError::from(e),
        ReportError::Invalid(msg) => ApiError::BadRequest(msg),
        ReportError::Render(msg) => ApiError::Internal(msg),
    }
}

#[derive(Serialize)]
pub struct ReportTypeInfo {
    pub key: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub formats: [&'static str; 2],
    pub sections: Vec<SectionSpec>,
    /// The built-in template, filled for this project and user.
    pub default_definition: ReportDefinition,
}

/// Report types, their sections (with option schemas) and default templates.
#[get("/projects/<project_id>/reports/types")]
pub async fn list_types(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<ReportTypeInfo>>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let ctx = default_context(state, access.user(), project_id)?;
    Ok(Json(
        ReportType::ALL
            .into_iter()
            .map(|t| ReportTypeInfo {
                key: t.key(),
                title: t.title(),
                description: t.description(),
                formats: ["pdf", "odt"],
                sections: catalog()
                    .into_iter()
                    .filter(|s| s.report_types.contains(&t))
                    .collect(),
                default_definition: ReportDefinition::default_for(t, &ctx),
            })
            .collect(),
    ))
}

/// A template as the API returns it.
#[derive(Serialize)]
pub struct TemplateView {
    pub id: i32,
    pub project_id: i32,
    pub name: String,
    pub report_type: String,
    pub visibility: String,
    pub owner_id: i32,
    pub owner_name: String,
    /// Normalised: sections added since it was saved are listed, disabled.
    pub definition: serde_json::Value,
    /// Whether the caller may change or delete it.
    pub can_edit: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

fn can_edit(state: &AppState, user: &User, t: &ReportTemplate) -> ApiResult<bool> {
    if crate::permissions::project_is_archived(&*state.repo_read(), t.project_id) {
        return Ok(false);
    }
    if user.is_admin || t.owner_id == user.id {
        return Ok(true);
    }
    let members = state
        .repo_read()
        .get_members_by_project(t.project_id)
        .map_err(ApiError::from)?;
    Ok(members
        .iter()
        .any(|m| m.user_id == user.id && m.role == ROLE_ADMIN))
}

fn visible(user: &User, t: &ReportTemplate) -> bool {
    t.visibility == "shared" || t.owner_id == user.id || user.is_admin
}

fn view(state: &AppState, user: &User, t: ReportTemplate) -> ApiResult<TemplateView> {
    let owner_name = state
        .repo_read()
        .get_user_by_id(t.owner_id)
        .map(|u| {
            if u.name.trim().is_empty() {
                u.username
            } else {
                u.name
            }
        })
        .unwrap_or_else(|_| format!("#{}", t.owner_id));
    let definition = serde_json::from_value::<ReportDefinition>(t.definition.clone())
        .ok()
        .and_then(|d| d.validated().ok())
        .and_then(|d| serde_json::to_value(d).ok())
        .unwrap_or_else(|| t.definition.clone());
    Ok(TemplateView {
        can_edit: can_edit(state, user, &t)?,
        id: t.id,
        project_id: t.project_id,
        name: t.name,
        report_type: t.report_type,
        visibility: t.visibility,
        owner_id: t.owner_id,
        owner_name,
        definition,
        created_at: t.created_at,
        updated_at: t.updated_at,
    })
}

/// A visible template of `project_id`, else 404.
fn find_template(
    state: &AppState,
    user: &User,
    project_id: i32,
    template_id: i32,
) -> ApiResult<ReportTemplate> {
    let t = state
        .repo_read()
        .get_report_template(template_id)
        .map_err(|_| ApiError::NotFound("report template not found".into()))?;
    if t.project_id != project_id || !visible(user, &t) {
        return Err(ApiError::NotFound("report template not found".into()));
    }
    Ok(t)
}

fn audit(
    state: &AppState,
    user: &User,
    action: ActionType,
    t: &ReportTemplate,
    old: Option<&ReportTemplate>,
) {
    let _ = state.repo_write().insert_log(&NewLog {
        user_id: user.id,
        action_type: action.to_string(),
        entity_type: EntityType::ReportTemplate.to_string(),
        entity_id: Some(t.id),
        project_id: Some(t.project_id),
        old_values: old.and_then(|o| serde_json::to_string(o).ok()),
        new_values: if matches!(action, ActionType::Delete) {
            None
        } else {
            serde_json::to_string(t).ok()
        },
        description: Some(format!(
            "{} report template \"{}\"",
            match action {
                ActionType::Create => "Created",
                ActionType::Delete => "Deleted",
                _ => "Updated",
            },
            t.name
        )),
        ip_address: None,
        user_agent: None,
    });
}

#[get("/projects/<project_id>/report_templates")]
pub async fn list_templates(
    access: ProjectAccessOrBearer,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<TemplateView>>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let rows = state
        .repo_read()
        .list_report_templates(project_id)
        .map_err(ApiError::from)?;
    let user = access.user();
    rows.into_iter()
        .filter(|t| visible(user, t))
        .map(|t| view(state, user, t))
        .collect::<ApiResult<Vec<_>>>()
        .map(Json)
}

#[get("/projects/<project_id>/report_templates/<template_id>")]
pub async fn get_template(
    access: ProjectAccessOrBearer,
    project_id: i32,
    template_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<TemplateView>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let t = find_template(state, access.user(), project_id, template_id)?;
    Ok(Json(view(state, access.user(), t)?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemplateInput {
    pub name: Option<String>,
    pub visibility: Option<String>,
    pub definition: Option<ReportDefinition>,
}

fn clean_name(name: &str) -> ApiResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > NAME_MAX {
        return Err(ApiError::BadRequest(format!(
            "name must be 1 to {NAME_MAX} characters"
        )));
    }
    Ok(name.to_string())
}

fn clean_visibility(v: &str) -> ApiResult<String> {
    match v {
        "private" | "shared" => Ok(v.to_string()),
        _ => Err(ApiError::BadRequest(
            "visibility must be private or shared".into(),
        )),
    }
}

fn clean_definition(def: ReportDefinition) -> ApiResult<serde_json::Value> {
    let def = def.validated().map_err(ApiError::BadRequest)?;
    serde_json::to_value(def).map_err(ApiError::from)
}

/// Save a template. Any member who can view the project may save one; shared
/// templates are visible to every member.
#[post("/projects/<project_id>/report_templates", data = "<body>")]
pub async fn create_template(
    access: ProjectAccessOrBearer,
    project_id: i32,
    body: Json<TemplateInput>,
    state: &State<AppState>,
) -> ApiResult<(Status, Json<TemplateView>)> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    // Archived projects are read-only (issue #381).
    require_not_archived(state, project_id)?;
    let body = body.into_inner();
    let (Some(name), Some(definition)) = (body.name.as_deref(), body.definition) else {
        return Err(ApiError::BadRequest(
            "name and definition are required".into(),
        ));
    };
    let report_type = definition.report_type.key().to_string();
    let write = ReportTemplateWrite {
        project_id,
        owner_id: access.user().id,
        name: clean_name(name)?,
        report_type,
        visibility: clean_visibility(body.visibility.as_deref().unwrap_or("private"))?,
        definition: clean_definition(definition)?,
        updated_at: chrono::Utc::now().naive_utc(),
    };
    let created = state
        .repo_write()
        .create_report_template(&write)
        .map_err(ApiError::from)?;
    audit(state, access.user(), ActionType::Create, &created, None);
    Ok((Status::Created, Json(view(state, access.user(), created)?)))
}

/// Change name, visibility or definition (owner, project Admin or instance
/// admin). The report type is fixed.
#[patch(
    "/projects/<project_id>/report_templates/<template_id>",
    data = "<body>"
)]
pub async fn update_template(
    access: ProjectAccessOrBearer,
    project_id: i32,
    template_id: i32,
    body: Json<TemplateInput>,
    state: &State<AppState>,
) -> ApiResult<Json<TemplateView>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    // Archived projects are read-only (issue #381).
    require_not_archived(state, project_id)?;
    let existing = find_template(state, access.user(), project_id, template_id)?;
    if !can_edit(state, access.user(), &existing)? {
        return Err(ApiError::Forbidden(
            "only the owner or a project admin can change this template".into(),
        ));
    }
    let body = body.into_inner();
    let definition = match body.definition {
        Some(d) if d.report_type.key() != existing.report_type => {
            return Err(ApiError::BadRequest(
                "the report type of a template cannot be changed".into(),
            ));
        }
        Some(d) => clean_definition(d)?,
        None => existing.definition.clone(),
    };
    let write = ReportTemplateWrite {
        project_id,
        owner_id: existing.owner_id,
        name: match body.name.as_deref() {
            Some(n) => clean_name(n)?,
            None => existing.name.clone(),
        },
        report_type: existing.report_type.clone(),
        visibility: match body.visibility.as_deref() {
            Some(v) => clean_visibility(v)?,
            None => existing.visibility.clone(),
        },
        definition,
        updated_at: chrono::Utc::now().naive_utc(),
    };
    let updated = state
        .repo_write()
        .update_report_template(template_id, &write)
        .map_err(ApiError::from)?;
    audit(
        state,
        access.user(),
        ActionType::Update,
        &updated,
        Some(&existing),
    );
    Ok(Json(view(state, access.user(), updated)?))
}

#[delete("/projects/<project_id>/report_templates/<template_id>")]
pub async fn delete_template(
    access: ProjectAccessOrBearer,
    project_id: i32,
    template_id: i32,
    state: &State<AppState>,
) -> ApiResult<Status> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    // Archived projects are read-only (issue #381).
    require_not_archived(state, project_id)?;
    let existing = find_template(state, access.user(), project_id, template_id)?;
    if !can_edit(state, access.user(), &existing)? {
        return Err(ApiError::Forbidden(
            "only the owner or a project admin can delete this template".into(),
        ));
    }
    state
        .repo_write()
        .delete_report_template(template_id)
        .map_err(ApiError::from)?;
    audit(
        state,
        access.user(),
        ActionType::Delete,
        &existing,
        Some(&existing),
    );
    Ok(Status::NoContent)
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct GenerateRequest {
    /// A saved template to start from.
    pub template_id: Option<i32>,
    /// An unsaved definition; wins over `template_id`.
    pub definition: Option<ReportDefinition>,
}

/// Generate a report document: `<file>` is `{vcd|coverage}.{pdf|odt}`. The
/// definition comes from the body, else the saved template, else the
/// built-in default.
#[post("/projects/<project_id>/reports/<file>", data = "<body>")]
pub async fn generate(
    access: ProjectAccessOrBearer,
    project_id: i32,
    file: &str,
    body: Option<Json<GenerateRequest>>,
    state: &State<AppState>,
) -> ApiResult<FileDownload> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let (kind, ext) = file
        .rsplit_once('.')
        .ok_or_else(|| ApiError::NotFound("unknown report".into()))?;
    let report_type =
        ReportType::parse(kind).ok_or_else(|| ApiError::NotFound("unknown report type".into()))?;
    let format = ReportFormat::parse(ext)
        .ok_or_else(|| ApiError::NotFound("unknown report format".into()))?;
    let body = body.map(Json::into_inner).unwrap_or_default();
    let definition = match (body.definition, body.template_id) {
        (Some(d), _) => d,
        (None, Some(id)) => {
            let t = find_template(state, access.user(), project_id, id)?;
            serde_json::from_value(t.definition)
                .map_err(|e| ApiError::BadRequest(format!("stored template is invalid: {e}")))?
        }
        (None, None) => default_definition(state, access.user(), project_id, report_type)?,
    };
    if definition.report_type != report_type {
        return Err(ApiError::BadRequest(format!(
            "the definition is for a {} report, not {}",
            definition.report_type.key(),
            report_type.key()
        )));
    }
    let owned = state.inner().clone();
    let report = rocket::tokio::task::spawn_blocking(move || {
        crate::reports::generate(&owned, project_id, definition, format)
    })
    .await
    .map_err(|e| ApiError::Internal(e.to_string()))?
    .map_err(report_error)?;
    Ok(FileDownload::new(
        report.bytes,
        report.content_type,
        report.filename,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::session::test_session_cookie_for;
    use crate::models::{Project, ProjectMember};
    use crate::permissions::{ROLE_AUTHOR, ROLE_VIEWER};
    use crate::repository::{CacheRepository, diesel_repo_mock::DieselRepoMock};
    use crate::status_enums::ProjectStatus;
    use chrono::NaiveDate;
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::{Value, json};
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const PROJECT: i32 = 5;
    const OTHER: i32 = 6;
    const AUTHOR: i32 = 1;
    const VIEWER: i32 = 2;
    const ADMIN: i32 = 3;
    const OUTSIDER: i32 = 4;

    fn repo() -> DieselRepoMock {
        let ts = NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let mut repo = DieselRepoMock::default();
        for (id, name) in [
            (AUTHOR, "arno"),
            (VIEWER, "vera"),
            (ADMIN, "ada"),
            (OUTSIDER, "otto"),
        ] {
            let mut user = DieselRepoMock::make_user(id, name, "");
            user.name = format!("{}{}", name[..1].to_uppercase(), &name[1..]);
            repo.users.insert(id, user);
        }
        for id in [PROJECT, OTHER] {
            repo.projects.insert(
                id,
                Project {
                    id,
                    name: format!("Project {id}"),
                    description: None,
                    creation_date: None,
                    update_date: None,
                    status: ProjectStatus::Active,
                    owner_id: Some(ADMIN),
                    slug: format!("project-{id}"),
                    group_id: None,
                    archived_at: None,
                    archived_by: None,
                },
            );
        }
        for (user_id, role) in [
            (AUTHOR, ROLE_AUTHOR),
            (VIEWER, ROLE_VIEWER),
            (ADMIN, ROLE_ADMIN),
        ] {
            repo.project_members.push(ProjectMember {
                project_id: PROJECT,
                user_id,
                role,
                created_at: ts,
                updated_at: ts,
            });
        }
        repo
    }

    async fn client() -> Client {
        let state = AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo(), 0))),
        };
        Client::tracked(rocket::build().manage(state).mount(
            "/api",
            routes![
                list_types,
                list_templates,
                get_template,
                create_template,
                update_template,
                delete_template,
                generate
            ],
        ))
        .await
        .expect("client")
    }

    struct Res {
        status: Status,
        content_type: Option<String>,
        disposition: Option<String>,
        bytes: Vec<u8>,
    }

    impl Res {
        fn json(&self) -> Value {
            serde_json::from_slice(&self.bytes).unwrap_or(Value::Null)
        }
    }

    async fn send(
        client: &Client,
        user: Option<i32>,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Res {
        let path = format!("/api/projects/{path}");
        let mut req = match method {
            "POST" => client.post(path),
            "PATCH" => client.patch(path),
            "DELETE" => client.delete(path),
            _ => client.get(path),
        };
        if let Some(body) = body {
            req = req.header(ContentType::JSON).body(body.to_string());
        }
        if let Some(user) = user {
            let state = client.rocket().state::<TestState>().unwrap();
            req = req.private_cookie(test_session_cookie_for(state, user));
        }
        let res = req.dispatch().await;
        Res {
            status: res.status(),
            content_type: res.headers().get_one("Content-Type").map(String::from),
            disposition: res
                .headers()
                .get_one("Content-Disposition")
                .map(String::from),
            bytes: res.into_bytes().await.unwrap_or_default(),
        }
    }

    fn vcd(sections: Value) -> Value {
        json!({ "report_type": "vcd", "sections": sections })
    }

    #[rocket::async_test]
    async fn types_list_sections_and_project_defaults() {
        let client = client().await;
        let res = send(&client, Some(VIEWER), "GET", "5/reports/types", None).await;
        assert_eq!(res.status, Status::Ok);
        let types = res.json();
        assert_eq!(types[0]["key"], "vcd");
        assert_eq!(types[1]["key"], "coverage");
        let keys: Vec<_> = types[0]["sections"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["key"].as_str().unwrap().to_string())
            .collect();
        assert!(keys.contains(&"matrix".to_string()));
        let matrix = types[0]["sections"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["key"] == "matrix")
            .unwrap();
        assert_eq!(matrix["options"][1]["kind"], "multi_select");
        assert_eq!(
            types[0]["default_definition"]["document"]["doc_id"],
            "PROJECT-5-VCD-001"
        );
        assert_eq!(
            types[0]["default_definition"]["document"]["signatories"][0]["name"], "Vera",
            "prepared by the caller"
        );
        let res = send(&client, Some(OUTSIDER), "GET", "5/reports/types", None).await;
        assert!(matches!(res.status.code, 403 | 404), "{}", res.status);
    }

    #[rocket::async_test]
    async fn templates_follow_visibility_and_edit_rules() {
        let client = client().await;
        let mine = send(
            &client,
            Some(AUTHOR),
            "POST",
            "5/report_templates",
            Some(json!({"name": " CDR VCD ", "definition": vcd(json!([{"key": "matrix"}]))})),
        )
        .await;
        assert_eq!(mine.status, Status::Created);
        let mine = mine.json();
        assert_eq!(mine["name"], "CDR VCD");
        assert_eq!(mine["visibility"], "private");
        assert_eq!(mine["report_type"], "vcd");
        assert_eq!(mine["can_edit"], true);
        assert_eq!(mine["owner_name"], "Arno");
        let sections = mine["definition"]["sections"].as_array().unwrap();
        assert_eq!(sections[0]["key"], "matrix");
        assert!(sections.len() > 1 && sections[1..].iter().all(|s| s["enabled"] == false));

        let shared = send(
            &client,
            Some(AUTHOR),
            "POST",
            "5/report_templates",
            Some(json!({"name": "Team", "visibility": "shared", "definition": vcd(json!([]))})),
        )
        .await
        .json();

        // The viewer sees only the shared one, and cannot change it.
        let list = send(&client, Some(VIEWER), "GET", "5/report_templates", None)
            .await
            .json();
        let names: Vec<_> = list
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].clone())
            .collect();
        assert_eq!(names, [json!("Team")]);
        assert_eq!(list[0]["can_edit"], false);
        let private = format!("5/report_templates/{}", mine["id"]);
        assert_eq!(
            send(&client, Some(VIEWER), "GET", &private, None)
                .await
                .status,
            Status::NotFound
        );
        let shared_path = format!("5/report_templates/{}", shared["id"]);
        let res = send(
            &client,
            Some(VIEWER),
            "PATCH",
            &shared_path,
            Some(json!({"name": "Mine now"})),
        )
        .await;
        assert_eq!(res.status, Status::Forbidden);

        // A project Admin may change and delete someone else's shared template.
        let res = send(
            &client,
            Some(ADMIN),
            "PATCH",
            &shared_path,
            Some(json!({"name": "Team VCD"})),
        )
        .await;
        assert_eq!(res.status, Status::Ok);
        assert_eq!(
            res.json()["visibility"],
            "shared",
            "unchanged fields are kept"
        );
        assert_eq!(res.json()["owner_id"], AUTHOR, "the owner stays");
        assert_eq!(
            send(&client, Some(ADMIN), "DELETE", &shared_path, None)
                .await
                .status,
            Status::NoContent
        );
        assert_eq!(
            send(&client, Some(ADMIN), "GET", &shared_path, None)
                .await
                .status,
            Status::NotFound
        );

        // Validation.
        let res = send(
            &client,
            Some(AUTHOR),
            "POST",
            "5/report_templates",
            Some(json!({"name": "cdr vcd", "definition": vcd(json!([]))})),
        )
        .await;
        assert_eq!(
            res.status,
            Status::Conflict,
            "names are unique per owner, ignoring case"
        );
        let res = send(
            &client,
            Some(AUTHOR),
            "POST",
            "5/report_templates",
            Some(json!({"name": "x", "definition": vcd(json!([{"key": "toc"}, {"key": "toc"}]))})),
        )
        .await;
        assert_eq!(res.status, Status::BadRequest);
        let res = send(
            &client,
            Some(AUTHOR),
            "POST",
            "5/report_templates",
            Some(json!({"name": "  ", "definition": vcd(json!([]))})),
        )
        .await;
        assert_eq!(res.status, Status::BadRequest);
        let res = send(
            &client,
            Some(AUTHOR),
            "POST",
            "5/report_templates",
            Some(json!({"name": "x", "visibility": "public", "definition": vcd(json!([]))})),
        )
        .await;
        assert_eq!(res.status, Status::BadRequest);
        let res = send(
            &client,
            Some(AUTHOR),
            "PATCH",
            &private,
            Some(json!({"definition": {"report_type": "coverage", "sections": []}})),
        )
        .await;
        assert_eq!(res.status, Status::BadRequest, "the report type is fixed");
        let res = send(
            &client,
            Some(AUTHOR),
            "GET",
            &format!("6/report_templates/{}", mine["id"]),
            None,
        )
        .await;
        assert!(
            matches!(res.status.code, 403 | 404),
            "another project: {}",
            res.status
        );

        let state = client.rocket().state::<TestState>().unwrap();
        let logs: Vec<_> = state
            .repo_read()
            .inner_repo()
            .logs
            .iter()
            .filter(|l| l.entity_type == "REPORT_TEMPLATE")
            .map(|l| l.description.clone().unwrap_or_default())
            .collect();
        assert_eq!(
            logs,
            [
                "Created report template \"CDR VCD\"",
                "Created report template \"Team\"",
                "Updated report template \"Team VCD\"",
                "Deleted report template \"Team VCD\""
            ]
        );
    }

    #[rocket::async_test]
    async fn generates_pdf_and_odt_from_defaults_templates_or_definitions() {
        let client = client().await;
        let res = send(&client, Some(VIEWER), "POST", "5/reports/vcd.pdf", None).await;
        assert_eq!(res.status, Status::Ok);
        assert_eq!(res.content_type.as_deref(), Some("application/pdf"));
        assert!(res.bytes.starts_with(b"%PDF-"));
        let disposition = res.disposition.unwrap();
        assert!(
            disposition.starts_with("attachment; filename=\"PROJECT-5-VCD-001-"),
            "{disposition}"
        );
        assert!(disposition.ends_with(".pdf\""));

        let res = send(
            &client,
            Some(VIEWER),
            "POST",
            "5/reports/coverage.odt",
            Some(json!({})),
        )
        .await;
        assert_eq!(res.status, Status::Ok);
        assert_eq!(
            res.content_type.as_deref(),
            Some("application/vnd.oasis.opendocument.text")
        );
        assert!(res.bytes.starts_with(b"PK"));

        let created = send(
            &client,
            Some(AUTHOR),
            "POST",
            "5/report_templates",
            Some(json!({"name": "Short", "definition": {"report_type": "coverage", "document": {"doc_id": "SHORT-1"}, "sections": [{"key": "coverage_summary"}]}})),
        )
        .await
        .json();
        let res = send(
            &client,
            Some(AUTHOR),
            "POST",
            "5/reports/coverage.odt",
            Some(json!({"template_id": created["id"]})),
        )
        .await;
        assert_eq!(res.status, Status::Ok);
        assert!(res.disposition.unwrap().contains("SHORT-1-"));
        let res = send(
            &client,
            Some(VIEWER),
            "POST",
            "5/reports/coverage.pdf",
            Some(json!({"template_id": created["id"]})),
        )
        .await;
        assert_eq!(
            res.status,
            Status::NotFound,
            "private template of another user"
        );

        let res = send(
            &client,
            Some(VIEWER),
            "POST",
            "5/reports/vcd.pdf",
            Some(json!({"definition": vcd(json!([{"key": "summary"}]))})),
        )
        .await;
        assert_eq!(res.status, Status::Ok, "unsaved definition");
        let res = send(
            &client,
            Some(VIEWER),
            "POST",
            "5/reports/coverage.pdf",
            Some(json!({"definition": vcd(json!([]))})),
        )
        .await;
        assert_eq!(
            res.status,
            Status::BadRequest,
            "definition type must match the URL"
        );
        let res = send(
            &client,
            Some(VIEWER),
            "POST",
            "5/reports/vcd.pdf",
            Some(json!({"definition": vcd(json!([{"key": "nope"}]))})),
        )
        .await;
        assert_eq!(res.status, Status::BadRequest);
        for path in ["5/reports/vcd.docx", "5/reports/pie.pdf", "5/reports/vcd"] {
            assert_eq!(
                send(&client, Some(VIEWER), "POST", path, None).await.status,
                Status::NotFound,
                "{path}"
            );
        }
        let res = send(&client, Some(OUTSIDER), "POST", "5/reports/vcd.pdf", None).await;
        assert!(matches!(res.status.code, 403 | 404), "{}", res.status);
        assert_eq!(
            send(&client, None, "POST", "5/reports/vcd.pdf", None)
                .await
                .status,
            Status::Unauthorized
        );
    }
}
