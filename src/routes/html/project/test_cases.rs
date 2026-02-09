use std::collections::HashMap;

use rocket::serde::json::Json;
use rocket::serde::json::Value;

use super::helpers::*;
use super::prelude::*;
use crate::helper_functions::decorators::decorate_requirements_with_repo;
use crate::models::EntityType;
use crate::services::{
    change_summary, log_change_details, resolve_change_details_labels, LabelResolvers, LogService,
    StatusService, TestService,
};
use crate::status_enums::TestStatusEnum;

/// Payload for inline status update (POST from tests list page). Accepts JSON for reliable parsing.
#[derive(rocket::serde::Deserialize)]
#[serde(crate = "rocket::serde")]
struct UpdateTestStatusForm {
    status_id: i32,
}

/// Returns only the four canonical test statuses (Passed, Failed, Pending, In Progress) for the project.
fn canonical_test_statuses(state: &AppState, project_id: i32) -> Vec<crate::models::TestStatus> {
    let statuses = StatusService::new(state)
        .list_test_statuses_by_project(project_id)
        .unwrap_or_default();
    let mut out: Vec<_> = statuses
        .into_iter()
        .filter(|s| TestStatusEnum::from_title(&s.title).is_some())
        .collect();
    out.sort_by_key(|s| {
        TestStatusEnum::from_title(&s.title)
            .map(|e| e.id())
            .unwrap_or(i32::MAX)
    });
    out
}

#[get("/<project_id>/tests?<status_filter>&<verification_filter>&<category_filter>&<search>")]
#[allow(clippy::too_many_arguments)]
async fn show_tests(
    project_access: ProjectAccess,
    project_id: i32,
    cookies: &CookieJar<'_>,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
    search: Option<String>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let user = project_access.into_user();
    let is_admin = user.is_admin;
    let service = TestService::new(state.inner());
    let repo = state.repo_read();

    let mut ctx = build_context_with_projects(state, user, cookies);

    // Get project info
    let project = repo.get_project_by_id(project_id).ok();
    if let Some(ref proj) = project {
        ctx["project"] = json!({
            "id": proj.id,
            "name": proj.name,
        });
    }

    // Fetch and process tests
    let all_tests = service.list_by_project(project_id).unwrap_or_default();

    // Calculate metrics before filtering
    // Using enum definitions for test statuses: Passed=1, Failed=2, Pending=3, InProgress=4
    let total = all_tests.len();
    let passed = all_tests
        .iter()
        .filter(|t| t.status_id == TestStatusEnum::Passed.id())
        .count();
    let failed = all_tests
        .iter()
        .filter(|t| t.status_id == TestStatusEnum::Failed.id())
        .count();
    let pending = all_tests
        .iter()
        .filter(|t| t.status_id == TestStatusEnum::Pending.id())
        .count();
    let in_progress = all_tests
        .iter()
        .filter(|t| t.status_id == TestStatusEnum::InProgress.id())
        .count();
    //let pass_rate_percent = if total > 0 { (passed * 100) / total } else { 0 };
    let pass_rate_percent = (passed * 100).checked_div(total).unwrap_or(0);

    // Apply filters
    let mut tests = filter_tests(
        all_tests,
        status_filter,
        verification_filter,
        category_filter,
    );

    // Apply search filter
    if let Some(ref query) = search {
        let query_lower = query.to_lowercase();
        tests.retain(|t| {
            t.name.to_lowercase().contains(&query_lower)
                || t.description.to_lowercase().contains(&query_lower)
                || t.reference_code.to_lowercase().contains(&query_lower)
        });
    }

    let tests = decorate_tests_cached(state, tests);
    ctx["tests"] = json!(tests);

    // Add metrics
    ctx["test_metrics"] = json!({
        "total": total,
        "passed": passed,
        "failed": failed,
        "pending": pending,
        "in_progress": in_progress,
        "pass_rate": {
            "percent": pass_rate_percent,
            "passed": passed
        }
    });

    // Common data lookups (for filters and inline edit). Only the four canonical statuses.
    let statuses = canonical_test_statuses(state.inner(), project_id);

    let verifications = repo
        .get_verification_by_project(project_id)
        .unwrap_or_default();
    let categories = repo
        .get_categories_by_project(project_id)
        .unwrap_or_default();

    let inline_edit_config = json!({
        "statuses": statuses.iter().map(|s| json!({"id": s.id, "title": s.title})).collect::<Vec<_>>(),
        "verifications": verifications.iter().map(|v| json!({"id": v.id, "title": v.title})).collect::<Vec<_>>(),
        "categories": categories.iter().map(|c| json!({"id": c.id, "title": c.title})).collect::<Vec<_>>(),
    });
    let inline_edit_config_json =
        serde_json::to_string(&inline_edit_config).unwrap_or_else(|_| "{}".to_string());

    ctx["statuses"] = json!(statuses);
    ctx["verifications"] = json!(verifications);
    ctx["categories"] = json!(categories);
    ctx["inline_edit_config_json"] = json!(inline_edit_config_json);

    // Active filter values
    ctx["current_status_filter"] = json!(status_filter);
    ctx["current_verification_filter"] = json!(verification_filter);
    ctx["current_category_filter"] = json!(category_filter);
    ctx["search_query"] = json!(search.unwrap_or_default());

    // User info for admin checks
    ctx["is_admin"] = json!(is_admin);

    // Add page title
    if let Some(proj) = project {
        ctx["page_title"] = json!(format!("{} - Tests", proj.name));
    } else {
        ctx["page_title"] = json!("Tests");
    }

    Ok(Template::render("tests/tests", ctx))
}

#[get("/<project_id>/tests/show/<test_id>")]
async fn show_test_id(
    project_access: ProjectAccess,
    project_id: i32,
    test_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let user = project_access.into_user();
    let service = TestService::new(state.inner());

    let test = match service.get_by_id(test_id) {
        Ok(t) => t,
        Err(details) => {
            let ctx = json!({
                "page_title": "Test Not Found",
                "message": "The test you're looking for could not be found.",
                "details": details.to_string(),
                "user": user
            });
            return Ok(Template::render("error", ctx));
        }
    };

    let decorated = decorate_tests_cached(state, vec![test]);
    let test = &decorated[0];

    let linked_requirements = get_requirements_for_test_cached(state, test_id).unwrap_or_default();
    let repo = state.repo_read();
    let decorated_requirements = decorate_requirements_with_repo(&*repo, linked_requirements);

    let history_entries = LogService::new(state.inner())
        .entity_logs(&EntityType::Test.to_string(), test_id)
        .unwrap_or_default();

    let repo = state.repo_read();
    let req_status_map: HashMap<i32, String> = repo
        .get_requirement_status_all()
        .unwrap_or_default()
        .into_iter()
        .map(|s| (s.id, s.title))
        .collect();
    let test_status_map: HashMap<i32, String> = repo
        .get_test_status_all()
        .unwrap_or_default()
        .into_iter()
        .map(|s| (s.id, s.title))
        .collect();
    let category_map: HashMap<i32, String> = repo
        .get_categories_by_project(project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|c| (c.id, c.title))
        .collect();
    let applicability_map: HashMap<i32, String> = repo
        .get_applicability_by_project(project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|a| (a.id, a.title))
        .collect();
    let verification_map: HashMap<i32, String> = repo
        .get_verification_by_project(project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|v| (v.id, v.title))
        .collect();
    let parent_label_map: HashMap<i32, String> = repo
        .get_tests_by_project(project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|t| (t.id, t.reference_code))
        .collect();
    drop(repo);

    let entries_with_summary: Vec<serde_json::Value> = history_entries
        .iter()
        .map(|e| {
            let mut v = serde_json::to_value(e).unwrap_or_else(|_| json!({}));
            if let Some(obj) = v.as_object_mut() {
                obj.insert("summary".into(), json!(change_summary(&e.log)));
                let details = log_change_details(&e.log);
                let resolvers = LabelResolvers {
                    req_status_map: &req_status_map,
                    test_status_map: &test_status_map,
                    category_map: &category_map,
                    applicability_map: &applicability_map,
                    verification_map: &verification_map,
                    parent_label_map: &parent_label_map,
                };
                let details = resolve_change_details_labels(details, "TEST", &resolvers);
                obj.insert("changes".into(), json!(details));
            }
            v
        })
        .collect();

    let mut ctx_map = serde_json::Map::new();
    ctx_map.insert("project_id".into(), json!(project_id));
    ctx_map.insert("selected_project_id".into(), json!(project_id));
    ctx_map.insert("linked_requirements".into(), json!(decorated_requirements));
    ctx_map.insert("user".into(), json!(user));
    ctx_map.insert("history".into(), json!({ "entries": entries_with_summary }));

    if let Ok(serde_json::Value::Object(test_obj)) = serde_json::to_value(test) {
        for (key, value) in test_obj {
            ctx_map.insert(key, value);
        }
    }

    // Add page title from test reference code
    if let Some(ref_code) = ctx_map.get("reference_code").and_then(|v| v.as_str()) {
        ctx_map.insert("page_title".into(), json!(format!("{} - Test", ref_code)));
    } else {
        ctx_map.insert("page_title".into(), json!("Test"));
    }

    Ok(Template::render(
        "tests/test",
        serde_json::Value::Object(ctx_map),
    ))
}

#[get("/<project_id>/tests/new?<error>")]
async fn new_test(
    project_access: ProjectAccess,
    project_id: i32,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let user = project_access.into_user();
    let repo = state.repo_read();

    let mut ctx = build_context_with_projects(state, user, cookies);
    ctx["categories"] = json!(repo
        .get_categories_by_project(project_id)
        .unwrap_or_default());
    ctx["status"] = json!(canonical_test_statuses(state.inner(), project_id));
    ctx["parents"] = json!(repo.get_tests_by_project(project_id).unwrap_or_default());
    ctx["users"] = json!(repo.get_users_all().unwrap_or_default());
    ctx["requirements"] = json!(repo
        .get_requirements_by_project(project_id)
        .unwrap_or_default());
    ctx["project_id"] = json!(project_id);
    ctx["selected_project_id"] = json!(project_id);
    ctx["error"] = json!(error);

    // Add page title
    if let Some(proj) = ctx
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
    {
        ctx["page_title"] = json!(format!("New Test - {}", proj));
    } else {
        ctx["page_title"] = json!("New Test");
    }

    Ok(Template::render("tests/new_test", ctx))
}

#[post("/<project_id>/tests/new", data = "<new_test>")]
async fn post_test(
    project_access: ProjectAccess,
    project_id: i32,
    new_test: Form<NewTestForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();
    let service = TestService::new(state.inner());

    let my_new_test = NewTestCase {
        id: None,
        name: new_test.name.clone(),
        description: new_test.description.clone(),
        source: new_test.source.clone(),
        status_id: new_test.status_id,
        reference_code: new_test.reference_code.clone(),
        parent_id: new_test.parent_id,
        project_id,
    };

    let id = service.create(&user, my_new_test).map_err(|e| {
        eprintln!("Error inserting new test: {:?}", e);
        Redirect::to(uri!(
            "/p",
            new_test(
                project_id = project_id,
                error = Some("Failed to create test".to_string())
            )
        ))
    })?;

    // Link requirements
    #[cfg(debug_assertions)]
    println!("NewTestForm requirements: {:#?}", new_test.test_req);
    for req in new_test.test_req.iter() {
        let matrix_item = NewMatrixLink {
            req_id: *req,
            test_id: id,
            project_id: new_test.project_id,
            triggering_version_id: None,
            triggering_user_id: None,
        };
        state
            .repo_write()
            .insert_new_matrix_item(&matrix_item)
            .map_err(|e| {
                eprintln!("Error inserting matrix item: {:?}", e);
                Redirect::to(uri!(
                    "/p",
                    new_test(
                        project_id = project_id,
                        error = Some("Failed to link requirements".to_string())
                    )
                ))
            })?;
    }

    Ok(Redirect::to(uri!("/p", show_test_id(project_id, id))))
}

#[get("/<project_id>/tests/edit/<test_id>")]
async fn get_edit_test(
    project_access: ProjectAccess,
    project_id: i32,
    test_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let user = project_access.into_user();
    let repo = state.repo_read();

    let test = match repo.get_test_by_id(test_id) {
        Ok(t) => t,
        Err(_) => return Err(Redirect::to(format!("/p/{}/tests", project_id))),
    };

    let decorated = decorate_tests_cached(state, vec![test]);
    let test0 = &decorated[0];

    let linked_requirements = get_requirements_for_test_cached(state, test_id).unwrap_or_default();
    let linked_req_ids: Vec<i32> = linked_requirements.iter().map(|r| r.id).collect();

    let ctx = json!({
        "tests": test0,
        "test_status_id": test0.test_status_id,
        "categories": repo.get_categories_by_project(project_id).unwrap_or_default(),
        "status": canonical_test_statuses(state.inner(), project_id),
        "parent": repo.get_tests_by_project(project_id).unwrap_or_default(),
        "users": repo.get_users_all().unwrap_or_default(),
        "verification": repo.get_verification_by_project(project_id).unwrap_or_default(),
        "linked_requirements": linked_requirements,
        "linked_req_ids": linked_req_ids,
        "requirements": repo.get_requirements_by_project(project_id).unwrap_or_default(),
        "user": user,
        "page_title": format!("Edit {} - Test", test0.reference_code)
    });

    #[cfg(debug_assertions)]
    println!("Tests: {:#}", ctx);

    Ok(Template::render("tests/edit_test", ctx))
}

#[post("/<project_id>/tests/edit/<test_id>", data = "<edit_test_form>")]
async fn post_edit_test(
    project_access: ProjectAccess,
    project_id: i32,
    test_id: i32,
    edit_test_form: Form<EditTestForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();
    let service = TestService::new(state.inner());
    let to_list = || Redirect::to(format!("/p/{}/tests", project_id));

    // Own the form to avoid cloning strings
    let f = edit_test_form.into_inner();

    let new_test = NewTestCase {
        id: Some(f.id),
        name: f.name,
        description: f.description,
        source: f.source,
        status_id: f.status_id,
        reference_code: f.reference_code,
        parent_id: f.parent_id,
        project_id: f.project_id,
    };

    service.update(&user, test_id, new_test).map_err(|e| {
        eprintln!("Error editing test: {e:?}");
        to_list()
    })?;

    state
        .repo_write()
        .update_test_requirement_links(f.id, &f.linked_requirements)
        .map_err(|e| {
            eprintln!("Error updating test requirement links: {e:?}");
            to_list()
        })?;

    Ok(Redirect::to(uri!("/p", show_test_id(project_id, f.id))))
}

#[delete("/<project_id>/tests/delete/<test_id>")]
async fn delete_test_route(
    project_access: ProjectAccess,
    project_id: i32,
    test_id: i32,
    state: &State<AppState>,
) -> Result<Redirect, rocket::http::Status> {
    use rocket::http::Status;

    let user = project_access.into_user();
    let service = TestService::new(state.inner());

    let test = service.get_by_id(test_id).map_err(|_| Status::NotFound)?;

    // Permission gate: only allow deletion of tests in Passed or Failed status, or if admin
    // Using enum to check if the test is in a deletable state
    let is_deletable = TestStatusEnum::from_id(test.status_id)
        .map(|status| matches!(status, TestStatusEnum::Passed | TestStatusEnum::Failed))
        .unwrap_or(false);

    if !is_deletable && !user.is_admin {
        return Err(Status::Forbidden);
    }

    service.delete(&user, test_id).map_err(|e| match e {
        crate::repository::errors::RepoError::NotFound => Status::NotFound,
        _ => Status::InternalServerError,
    })?;

    Ok(Redirect::to(format!("/p/{}/tests", project_id)))
}

/// POST /p/<project_id>/tests/update-status/<test_id> — inline status update (uses same session as page).
/// Accepts JSON body: { "status_id": 1 } for reliable parsing.
#[post("/<project_id>/tests/update-status/<test_id>", data = "<payload>")]
async fn update_test_status_route(
    project_access: ProjectAccess,
    project_id: i32,
    test_id: i32,
    payload: Json<UpdateTestStatusForm>,
    state: &State<AppState>,
) -> Result<Json<Value>, (rocket::http::Status, String)> {
    use rocket::http::Status;

    let user = project_access.into_user();
    let service = TestService::new(state.inner());

    let test = service
        .get_by_id(test_id)
        .map_err(|_| (Status::NotFound, "Test not found".to_string()))?;

    if test.project_id != project_id {
        return Err((
            Status::Forbidden,
            "Test does not belong to this project".to_string(),
        ));
    }

    let status_id = payload.status_id;
    let updated = NewTestCase {
        id: Some(test.id),
        reference_code: test.reference_code,
        name: test.name,
        description: test.description,
        source: test.source,
        status_id,
        parent_id: test.parent_id,
        project_id: test.project_id,
    };

    service.update(&user, test_id, updated).map_err(|e| {
        eprintln!("Error updating test status: {:?}", e);
        (Status::InternalServerError, "Update failed".to_string())
    })?;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[get("/<project_id>/requirements.xls")]
async fn get_requirements_xls(
    project_access: ProjectAccess,
    project_id: i32,
) -> Result<(ContentType, NamedFile), Redirect> {
    let user = project_access.into_user();
    println!(
        "User [{} - id:{}] requested requirements export for project_id={}",
        user.username, user.id, project_id
    );

    excel::create_requirements_workbook(project_id).map_err(|e| {
        eprintln!("Error creating requirements workbook: {e:?}");
        Redirect::to(format!("/p/{}/requirements", project_id))
    })?;
    let path_to_file = path::Path::new("target/requirements.xls");
    let file = NamedFile::open(&path_to_file).await.map_err(|e| {
        eprintln!("Error opening requirements export file: {e:?}");
        Redirect::to(format!("/p/{}/requirements", project_id))
    })?;
    let content_type = ContentType::new(
        "application",
        "vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    );
    Ok((content_type, file))
}

#[get("/<project_id>/tests.xls")]
async fn get_tests_xls(
    project_access: ProjectAccess,
    project_id: i32,
) -> Result<(ContentType, NamedFile), Redirect> {
    let user = project_access.into_user();
    println!(
        "User [{} - id:{}] requested tests export for project_id={}",
        user.username, user.id, project_id
    );
    excel::create_tests_workbook(project_id).map_err(|e| {
        eprintln!("Error creating tests workbook: {e:?}");
        Redirect::to(format!("/p/{}/tests", project_id))
    })?;
    let path_to_file = path::Path::new("target/tests.xls");
    let file = NamedFile::open(&path_to_file).await.map_err(|e| {
        eprintln!("Error opening tests export file: {e:?}");
        Redirect::to(format!("/p/{}/tests", project_id))
    })?;
    let content_type = ContentType::new(
        "application",
        "vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    );
    Ok((content_type, file))
}

pub fn routes() -> Vec<Route> {
    routes![
        delete_test_route,
        update_test_status_route,
        show_tests,
        show_test_id,
        new_test,
        get_edit_test,
        post_edit_test,
        post_test,
        get_requirements_xls,
        get_tests_xls
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Applicability, Category, MatrixLink, Project, ProjectMember, Requirement,
        RequirementStatus, TestCase, TestStatus, VerificationMethod,
    };
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::routes::html::project::test_helpers::{
        client_with_routes, delete_with_session, get_with_session, post_form_with_session,
        timestamp, TestAppState,
    };
    use crate::status_enums::ProjectStatus;
    use rocket::http::Status as HttpStatus;
    use rocket::local::asynchronous::Client;

    const ADMIN_ID: i32 = 1;
    const USER_ID: i32 = 2;
    const PRIMARY_PROJECT: i32 = 1;

    fn sample_project(id: i32, name: &str) -> Project {
        Project {
            id,
            name: name.to_string(),
            description: Some(format!("{name} project")),
            creation_date: Some(timestamp()),
            update_date: Some(timestamp()),
            status: ProjectStatus::Active,
            owner_id: Some(ADMIN_ID),
        }
    }

    fn sample_category(id: i32, title: &str) -> Category {
        Category {
            id,
            title: title.to_string(),
            description: format!("{title} systems"),
            tag: title.to_ascii_uppercase(),
            project_id: PRIMARY_PROJECT,
        }
    }

    fn sample_status(id: i32, title: &str) -> RequirementStatus {
        RequirementStatus {
            id,
            title: title.to_string(),
            description: format!("{title} status"),
            tag: title.to_ascii_uppercase(),
            project_id: 1,
        }
    }

    fn sample_test_status(id: i32, title: &str) -> TestStatus {
        TestStatus {
            id,
            title: title.to_string(),
            description: format!("{title} status"),
            tag: title.to_ascii_uppercase(),
            project_id: 1,
        }
    }

    fn sample_applicability(id: i32, title: &str) -> Applicability {
        Applicability {
            id,
            title: title.to_string(),
            description: format!("{title} applicability"),
            tag: title.to_ascii_uppercase(),
            project_id: PRIMARY_PROJECT,
        }
    }

    fn sample_verification(id: i32, title: &str) -> VerificationMethod {
        VerificationMethod {
            id,
            title: title.to_string(),
            description: format!("{title} verification"),
            tag: title.to_uppercase().replace(" ", "_"),
            project_id: PRIMARY_PROJECT,
        }
    }

    fn sample_requirement(id: i32) -> Requirement {
        Requirement {
            id,
            current_version_id: None,
            title: format!("Requirement {id}"),
            description: "Test requirement".into(),
            status_id: 1,
            author_id: ADMIN_ID,
            reviewer_id: ADMIN_ID,
            reference_code: format!("REQ-SYS-{id}"),
            category_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            applicability_id: 1,
            justification: Some("For testing".into()),
            project_id: PRIMARY_PROJECT,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
        }
    }

    fn sample_test(id: i32, status: i32, name: &str) -> TestCase {
        TestCase {
            id,
            name: name.to_string(),
            description: format!("{name} description"),
            source: "Design Spec".into(),
            status_id: status,
            reference_code: format!("TEST-{id:03}"),
            parent_id: None,
            project_id: PRIMARY_PROJECT,
        }
    }

    fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        let mut admin = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
        admin.is_admin = true;
        repo.users.insert(ADMIN_ID, admin);

        let mut user = DieselRepoMock::make_user(USER_ID, "user", "");
        user.is_admin = false;
        repo.users.insert(USER_ID, user);

        repo.projects
            .insert(PRIMARY_PROJECT, sample_project(PRIMARY_PROJECT, "Orbiter"));

        repo.project_members.push(ProjectMember {
            project_id: PRIMARY_PROJECT,
            user_id: ADMIN_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo.project_members.push(ProjectMember {
            project_id: PRIMARY_PROJECT,
            user_id: USER_ID,
            role: 3,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        repo.statuses.insert(1, sample_status(1, "Planned"));
        repo.test_statuses.insert(1, sample_test_status(1, "Draft"));
        repo.test_statuses
            .insert(2, sample_test_status(2, "Proposal"));
        repo.test_statuses
            .insert(3, sample_test_status(3, "Active"));

        repo.categories.insert(1, sample_category(1, "Systems"));
        repo.verifications
            .insert(1, sample_verification(1, "Analysis"));
        repo.applicability.insert(1, sample_applicability(1, "All"));
        repo.requirements.insert(1, sample_requirement(1));

        repo
    }

    fn repo_with_tests() -> DieselRepoMock {
        let mut repo = base_repo();
        repo.tests.insert(1, sample_test(1, 1, "Baseline Test"));
        repo.matrices.push(MatrixLink {
            req_id: 1,
            test_id: 1,
            creation_date: timestamp(),
            project_id: PRIMARY_PROJECT,
            suspect: false,
            suspect_at: None,
            suspect_reason: None,
            cleared_by: None,
            cleared_at: None,
            triggering_version_id: None,
            triggering_user_id: None,
        });
        repo
    }

    fn repo_with_active_test() -> DieselRepoMock {
        let mut repo = base_repo();
        repo.tests
            .insert(1, sample_test(1, 3, "Qualification Test"));
        repo
    }

    async fn test_client(repo: DieselRepoMock) -> Client {
        client_with_routes(
            repo,
            routes![
                show_tests,
                show_test_id,
                new_test,
                post_test,
                get_edit_test,
                post_edit_test,
                delete_test_route
            ],
        )
        .await
    }

    #[rocket::async_test]
    async fn show_tests_lists_known_items() {
        let client = test_client(repo_with_tests()).await;
        let response = get_with_session(&client, "/p/1/tests", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Baseline Test"));
    }

    #[rocket::async_test]
    async fn show_test_id_displays_details() {
        let client = test_client(repo_with_tests()).await;
        let response = get_with_session(&client, "/p/1/tests/show/1", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Baseline Test"));
        assert!(body.contains("description"));
    }

    #[rocket::async_test]
    async fn show_test_id_returns_error_when_missing() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/tests/show/42", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Test Not Found"));
    }

    #[rocket::async_test]
    async fn new_test_form_renders() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/tests/new", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("New Test"));
        assert!(body.contains("Create Test"));
    }

    #[rocket::async_test]
    async fn post_test_creates_new_entry() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/tests/new",
            concat!(
                "name=Thermal+Check&reference_code=TEST-002&description=Thermal+validation&",
                "source=Spec&status_id=1&parent_id=0&test_req=1&project_id=1"
            ),
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), HttpStatus::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/tests/show/1")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let inner = repo.inner_repo();

        let test = inner.tests.get(&1).expect("inserted test");
        assert_eq!(test.name, "Thermal Check");
        assert_eq!(test.status_id, 1);

        let links: Vec<_> = inner.matrices.iter().filter(|m| m.test_id == 1).collect();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].req_id, 1);
    }

    #[rocket::async_test]
    async fn get_edit_test_renders_existing_data() {
        let client = test_client(repo_with_tests()).await;
        let response = get_with_session(&client, "/p/1/tests/edit/1", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Edit Test"));
        assert!(body.contains("Baseline Test"));
    }

    #[rocket::async_test]
    async fn post_edit_test_updates_entry() {
        let client = test_client(repo_with_tests()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/tests/edit/1",
            concat!(
                "id=1&reference_code=TEST-001&name=Updated+Test&description=Updated+desc&",
                "source=Updated&status_id=2&parent_id=0&linked_requirements=1&project_id=1"
            ),
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), HttpStatus::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/tests/show/1")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let inner = repo.inner_repo();

        let test = inner.tests.get(&1).expect("existing test");
        assert_eq!(test.name, "Updated Test");
        assert_eq!(test.status_id, 2);

        let links: Vec<_> = inner.matrices.iter().filter(|m| m.test_id == 1).collect();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].req_id, 1);
    }

    #[rocket::async_test]
    async fn delete_test_route_removes_draft() {
        let client = test_client(repo_with_tests()).await;
        let response = delete_with_session(&client, "/p/1/tests/delete/1", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::SeeOther);
        let location = response.headers().get_one("Location");
        assert!(location.is_some());
        assert!(location.unwrap().contains("/p/1/tests"));

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        assert!(repo.inner_repo().tests.is_empty());
    }

    #[rocket::async_test]
    async fn delete_test_route_forbids_non_admin_when_status_high() {
        let client = test_client(repo_with_active_test()).await;
        let response = delete_with_session(&client, "/p/1/tests/delete/1", USER_ID).await;

        assert_eq!(response.status(), HttpStatus::Forbidden);
    }

    #[rocket::async_test]
    async fn show_tests_requires_membership_for_non_admin() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/tests", USER_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
    }
}
