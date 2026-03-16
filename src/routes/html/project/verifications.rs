// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use std::collections::HashMap;

use rocket::serde::json::Json;
use rocket::serde::json::Value;

use super::helpers::*;
use super::prelude::*;
use super::requirements;
use crate::helper_functions::decorators::decorate_requirements_with_repo;
use crate::models::EntityType;
use crate::services::{
    change_summary, log_change_details, resolve_change_details_labels, BaselineService,
    LabelResolvers, LogService, StatusService, VerificationService,
};
use crate::status_enums::TestStatusEnum;

const VERIFICATIONS_PER_PAGE: u64 = 25;

/// Build query string for verifications list (filters), without `page`.
fn build_verifications_query(
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
    search: Option<&str>,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(id) = status_filter {
        parts.push(format!("status_filter={}", id));
    }
    if let Some(id) = verification_filter {
        parts.push(format!("verification_filter={}", id));
    }
    if let Some(id) = category_filter {
        parts.push(format!("category_filter={}", id));
    }
    if let Some(s) = search.filter(|s| !s.is_empty()) {
        parts.push(format!("search={}", urlencoding::encode(s)));
    }
    parts.join("&")
}

fn verifications_list_path(project_slug: &str) -> String {
    format!("/p/{project_slug}/verifications")
}

fn verifications_list_redirect(project_slug: &str) -> Redirect {
    Redirect::to(verifications_list_path(project_slug))
}

fn test_detail_path(project_slug: &str, test_id: i32) -> String {
    format!("/p/{project_slug}/verifications/show/{test_id}")
}

fn edit_test_path(project_slug: &str, test_id: i32) -> String {
    format!("/p/{project_slug}/verifications/edit/{test_id}")
}

fn edit_test_panel_path(project_slug: &str, test_id: i32) -> String {
    format!("/p/{project_slug}/verifications/edit-panel/{test_id}")
}

fn new_test_path(project_slug: &str, error: Option<&str>) -> String {
    match error.filter(|value| !value.is_empty()) {
        Some(error) => format!(
            "/p/{project_slug}/verifications/new?error={}",
            urlencoding::encode(error)
        ),
        None => format!("/p/{project_slug}/verifications/new"),
    }
}

fn redirect_for_test_project(
    state: &State<AppState>,
    route_project_id: i32,
    resource_project_id: i32,
    test_id: i32,
) -> Option<Redirect> {
    if route_project_id == resource_project_id {
        None
    } else {
        let project_slug = get_project_slug_by_id_pooled_safe(state, resource_project_id);
        Some(Redirect::to(test_detail_path(&project_slug, test_id)))
    }
}

/// Payload for inline status update (POST from tests list page). Accepts JSON for reliable parsing.
#[derive(rocket::serde::Deserialize)]
#[serde(crate = "rocket::serde")]
struct UpdateTestStatusForm {
    status_id: i32,
}

/// Returns all test statuses for the project, with canonical four first (Passed, Failed, Pending, In Progress), then the rest by id.
/// Use for dropdowns, filters, and inline edit so user-created statuses are shown.
fn project_test_statuses(
    state: &AppState,
    project_id: i32,
) -> Vec<crate::models::VerificationStatus> {
    let statuses = StatusService::new(state)
        .list_verification_statuses_by_project(project_id)
        .unwrap_or_default();
    let (canonical, rest): (Vec<_>, Vec<_>) = statuses
        .into_iter()
        .partition(|s| TestStatusEnum::from_title(&s.title).is_some());
    let mut canonical = canonical;
    canonical.sort_by_key(|s| {
        TestStatusEnum::from_title(&s.title)
            .map(|e| e.canonical_order())
            .unwrap_or(i32::MAX)
    });
    let mut rest = rest;
    rest.sort_by_key(|s| s.id);
    canonical.extend(rest);
    canonical
}

#[get(
    "/<project_id>/verifications?<status_filter>&<verification_filter>&<category_filter>&<search>&<page>"
)]
#[allow(clippy::too_many_arguments)]
async fn show_tests(
    project_access: HtmlProjectAccess,
    project_id: String,
    cookies: &CookieJar<'_>,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
    search: Option<String>,
    page: Option<u32>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let is_admin = user.is_admin;
    let service = VerificationService::new(state.inner());
    let repo = state.repo_read();

    let mut ctx = build_context_with_projects(state, user.clone(), cookies);
    ctx["selected_project_id"] = json!(project_id);
    ctx["selected_project_slug"] = json!(project_slug);

    // Get project info
    let project = repo.get_project_by_id(project_id).ok();
    if let Some(ref proj) = project {
        ctx["project"] = json!({
            "id": proj.id,
            "name": proj.name,
            "slug": proj.slug,
        });
    }

    // Fetch and process tests
    let all_tests = service.list_by_project(project_id).unwrap_or_default();

    // Calculate metrics before filtering.
    // Resolve status title like the list decorator: project map first, then fallback to global
    // get_verification_status_by_id so counts match what is displayed.
    let test_status_titles: std::collections::HashMap<i32, String> =
        StatusService::new(state.inner()).verification_status_id_to_title_map(project_id);
    let missing_status_ids: std::collections::HashSet<i32> = all_tests
        .iter()
        .map(|t| t.status_id)
        .filter(|id| !test_status_titles.contains_key(id))
        .collect();
    let mut fallback_titles: std::collections::HashMap<i32, String> =
        std::collections::HashMap::new();
    for id in missing_status_ids {
        if let Ok(s) = repo.get_verification_status_by_id(id) {
            fallback_titles.insert(id, s.title);
        }
    }
    let status_title = |status_id: i32| -> Option<&str> {
        test_status_titles
            .get(&status_id)
            .or(fallback_titles.get(&status_id))
            .map(String::as_str)
    };
    let total = all_tests.len();
    let passed = all_tests
        .iter()
        .filter(|t| {
            status_title(t.status_id).and_then(TestStatusEnum::from_title)
                == Some(TestStatusEnum::Passed)
        })
        .count();
    let failed = all_tests
        .iter()
        .filter(|t| {
            status_title(t.status_id).and_then(TestStatusEnum::from_title)
                == Some(TestStatusEnum::Failed)
        })
        .count();
    let pending = all_tests
        .iter()
        .filter(|t| {
            status_title(t.status_id).and_then(TestStatusEnum::from_title)
                == Some(TestStatusEnum::Pending)
        })
        .count();
    let in_progress = all_tests
        .iter()
        .filter(|t| {
            status_title(t.status_id).and_then(TestStatusEnum::from_title)
                == Some(TestStatusEnum::InProgress)
        })
        .count();
    // Verifications whose status is not one of the canonical four (e.g. custom status) so metrics sum to total
    let other = total
        .saturating_sub(passed)
        .saturating_sub(failed)
        .saturating_sub(pending)
        .saturating_sub(in_progress);
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

    let total_count = tests.len() as u64;
    let per_page = VERIFICATIONS_PER_PAGE;
    let total_pages = if total_count == 0 {
        1
    } else {
        total_count.div_ceil(per_page)
    };
    let total_pages_u32 = total_pages.min(u32::MAX as u64) as u32;
    let current_page = page.unwrap_or(1).max(1).min(total_pages_u32);
    let offset = ((current_page - 1) as u64 * per_page) as usize;
    let page_tests: Vec<_> = tests
        .iter()
        .skip(offset)
        .take(per_page as usize)
        .cloned()
        .collect();
    let tests = decorate_tests_cached(state, page_tests);
    ctx["tests"] = json!(tests);

    let query_str = build_verifications_query(
        status_filter,
        verification_filter,
        category_filter,
        search.as_deref(),
    );
    ctx["pagination"] = json!(requirements::build_pagination_ctx(
        current_page,
        total_pages,
        total_count,
        per_page,
        &query_str,
    ));
    ctx["pagination_path"] = json!("verifications");

    // Add metrics (passed + failed + pending + in_progress + other = total)
    ctx["test_metrics"] = json!({
        "total": total,
        "passed": passed,
        "failed": failed,
        "pending": pending,
        "in_progress": in_progress,
        "other": other,
        "pass_rate": {
            "percent": pass_rate_percent,
            "passed": passed
        }
    });

    // Common data lookups (for filters and inline edit). All project statuses so user-created ones appear.
    let statuses = project_test_statuses(state.inner(), project_id);

    let verifications = repo
        .get_verification_methods_by_project(project_id)
        .unwrap_or_default();
    let categories = repo
        .get_categories_by_project(project_id)
        .unwrap_or_default();

    let inline_edit_config = json!({
        "statuses": statuses.iter().map(|s| json!({"id": s.id, "title": s.title, "tag_color": s.tag_color})).collect::<Vec<_>>(),
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

    if let Some(ctx_obj) = ctx.as_object_mut() {
        let perms = super::helpers::project_permissions_context(state, &user, project_id);
        if let Some(perms_obj) = perms.as_object() {
            for (k, v) in perms_obj {
                ctx_obj.insert(k.clone(), v.clone());
            }
        }
    }

    Ok(Template::render("verifications/verifications", ctx))
}

#[get("/<project_id>/verifications/show/<test_id>")]
async fn show_test_id(
    project_access: HtmlProjectAccess,
    project_id: String,
    test_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let service = VerificationService::new(state.inner());

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
    if let Some(redir) = redirect_for_test_project(state, project_id, test.project_id, test_id) {
        return Err(redir);
    }

    let linked_requirements = get_requirements_for_test_cached(state, test_id).unwrap_or_default();
    let repo = state.repo_read();
    let decorated_requirements = decorate_requirements_with_repo(&*repo, linked_requirements);

    let history_entries = LogService::new(state.inner())
        .entity_logs(&EntityType::Verification.to_string(), test_id)
        .unwrap_or_default();

    let repo = state.repo_read();
    let req_status_map: HashMap<i32, String> = repo
        .get_requirement_status_all()
        .unwrap_or_default()
        .into_iter()
        .map(|s| (s.id, s.title))
        .collect();
    let test_status_map: HashMap<i32, String> = repo
        .get_verification_status_all()
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
        .get_verification_methods_by_project(project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|v| (v.id, v.title))
        .collect();
    let parent_label_map: HashMap<i32, String> = repo
        .get_verifications_by_project(project_id)
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
    ctx_map.insert("project_slug".into(), json!(project_slug));
    ctx_map.insert("selected_project_id".into(), json!(project_id));
    ctx_map.insert(
        "selected_project_slug".into(),
        json!(get_project_slug_by_id_pooled_safe(state, project_id)),
    );
    ctx_map.insert("linked_requirements".into(), json!(decorated_requirements));
    ctx_map.insert("user".into(), json!(user));
    ctx_map.insert("history".into(), json!({ "entries": entries_with_summary }));

    if let Ok(serde_json::Value::Object(test_obj)) = serde_json::to_value(test) {
        for (key, value) in test_obj {
            ctx_map.insert(key, value);
        }
    }

    let verification_type_title = test
        .verification_method_id
        .and_then(|id| verification_map.get(&id).cloned())
        .unwrap_or_default();
    ctx_map.insert(
        "verification_type_title".into(),
        json!(verification_type_title),
    );

    // Add page title from test reference code
    if let Some(ref_code) = ctx_map.get("reference_code").and_then(|v| v.as_str()) {
        ctx_map.insert("page_title".into(), json!(format!("{} - Test", ref_code)));
    } else {
        ctx_map.insert("page_title".into(), json!("Test"));
    }

    let perms = super::helpers::project_permissions_context(state, &user, project_id);
    if let Some(perms_obj) = perms.as_object() {
        for (k, v) in perms_obj {
            ctx_map.insert(k.clone(), v.clone());
        }
    }

    Ok(Template::render(
        "verifications/verification",
        serde_json::Value::Object(ctx_map),
    ))
}

/// Show verification as at baseline time (same URL schema as requirements: .../show/<id>/version/<version_or_baseline_id>).
#[get("/<project_id>/verifications/show/<verification_id>/version/<baseline_id>")]
async fn show_verification_version(
    project_access: HtmlProjectAccess,
    project_id: String,
    verification_id: i32,
    baseline_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let baseline_service = BaselineService::new(state.inner());
    let baseline = match baseline_service.get_by_id(baseline_id) {
        Ok(b) => b,
        Err(_) => return Err(Redirect::to(format!("/p/{project_slug}/baselines"))),
    };
    if baseline.project_id != project_id {
        return Err(Redirect::to(format!("/p/{project_slug}/baselines")));
    }
    let snapshots = baseline_service
        .get_verifications(baseline_id)
        .unwrap_or_default();
    let snapshot = snapshots
        .iter()
        .find(|s| s.verification_id == verification_id);
    let snapshot = match snapshot {
        Some(s) => s,
        None => {
            return Err(Redirect::to(test_detail_path(
                &project_slug,
                verification_id,
            )));
        }
    };

    let repo = state.repo_read();
    let status_title = repo
        .get_verification_status_by_id(snapshot.status_id)
        .ok()
        .map(|s| s.title)
        .unwrap_or_else(|| format!("Status #{}", snapshot.status_id));
    let verification_type_title = snapshot
        .verification_method_id
        .and_then(|id| repo.get_verification_method_by_id(id).ok())
        .map(|m| m.title)
        .unwrap_or_default();
    let parent_title = snapshot
        .parent_id
        .and_then(|id| repo.get_verification_by_id(id).ok())
        .map(|v| v.reference_code)
        .unwrap_or_default();
    drop(repo);

    let mut ctx_map = serde_json::Map::new();
    ctx_map.insert("id".into(), json!(snapshot.verification_id));
    ctx_map.insert(
        "reference_code".into(),
        json!(snapshot.reference_code.clone()),
    );
    ctx_map.insert("name".into(), json!(snapshot.name.clone()));
    ctx_map.insert("description".into(), json!(snapshot.description.clone()));
    ctx_map.insert("source".into(), json!(snapshot.source.clone()));
    ctx_map.insert("status_id".into(), json!(status_title));
    ctx_map.insert("status_variant".into(), json!("default"));
    ctx_map.insert("verification_status_id".into(), json!(snapshot.status_id));
    ctx_map.insert(
        "verification_method_id".into(),
        json!(snapshot.verification_method_id),
    );
    ctx_map.insert(
        "verification_method_title".into(),
        json!(verification_type_title),
    );
    ctx_map.insert("verification_parent_id".into(), json!(snapshot.parent_id));
    ctx_map.insert("verification_parent_title".into(), json!(parent_title));
    ctx_map.insert("verification_parent_reference_code".into(), json!(""));
    ctx_map.insert("verification_parent_description".into(), json!(""));
    ctx_map.insert("verification_parent_status_id".into(), json!(""));
    ctx_map.insert(
        "verification_parent_status_variant".into(),
        json!("default"),
    );
    ctx_map.insert(
        "verification_parent_status_tag_color".into(),
        json!(Option::<String>::None),
    );
    ctx_map.insert("verification_parent_source".into(), json!(""));
    ctx_map.insert("project_id".into(), json!(snapshot.project_id));
    ctx_map.insert("project_slug".into(), json!(project_slug.clone()));
    ctx_map.insert("selected_project_id".into(), json!(project_id));
    ctx_map.insert("selected_project_slug".into(), json!(project_slug));
    ctx_map.insert("linked_requirements".into(), json!([]));
    ctx_map.insert("history".into(), json!({ "entries": [] }));
    ctx_map.insert("can_edit_requirements".into(), json!(false));
    ctx_map.insert("baseline_view".into(), json!(true));
    ctx_map.insert("baseline_id".into(), json!(baseline_id));
    ctx_map.insert("baseline_name".into(), json!(baseline.name.clone()));
    ctx_map.insert(
        "page_title".into(),
        json!(format!("{} (baseline) - Test", snapshot.reference_code)),
    );
    ctx_map.insert("user".into(), json!(user));

    let perms = super::helpers::project_permissions_context(state, &user, project_id);
    if let Some(perms_obj) = perms.as_object() {
        for (k, v) in perms_obj {
            ctx_map.insert(k.clone(), v.clone());
        }
    }

    Ok(Template::render(
        "verifications/verification",
        serde_json::Value::Object(ctx_map),
    ))
}

#[get("/<project_id>/verifications/new?<error>")]
async fn new_test(
    project_access: HtmlProjectAccess,
    project_id: String,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let repo = state.repo_read();

    let mut ctx = build_context_with_projects(state, user, cookies);
    ctx["categories"] = json!(repo
        .get_categories_by_project(project_id)
        .unwrap_or_default());
    let pending_title = TestStatusEnum::Pending.title();
    let status_with_default: Vec<serde_json::Value> =
        project_test_statuses(state.inner(), project_id)
            .into_iter()
            .map(|s| {
                json!({
                    "id": s.id,
                    "title": s.title,
                    "selected": s.title == pending_title
                })
            })
            .collect();
    ctx["status"] = json!(status_with_default);
    ctx["parents"] = json!(repo
        .get_verifications_by_project(project_id)
        .unwrap_or_default());
    ctx["users"] = json!(repo.get_users_all().unwrap_or_default());
    ctx["requirements"] = json!(repo
        .get_requirements_by_project(project_id)
        .unwrap_or_default());
    ctx["verification"] = json!(repo
        .get_verification_methods_by_project(project_id)
        .unwrap_or_default());
    ctx["project_id"] = json!(project_id);
    ctx["selected_project_id"] = json!(project_id);
    ctx["project_slug"] = json!(project_slug);
    ctx["selected_project_slug"] = json!(get_project_slug_by_id_pooled_safe(state, project_id));
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

    Ok(Template::render("verifications/new_verification", ctx))
}

#[post("/<project_id>/verifications/new", data = "<new_test>")]
async fn post_test(
    project_access: HtmlProjectAccess,
    project_id: String,
    new_test: Form<NewVerificationForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let service = VerificationService::new(state.inner());

    let my_new_verification = NewVerification {
        id: None,
        name: new_test.name.clone(),
        description: new_test.description.clone(),
        source: new_test.source.clone(),
        status_id: new_test.status_id,
        reference_code: new_test.reference_code.clone(),
        parent_id: new_test
            .parent_id
            .and_then(|id| if id == 0 { None } else { Some(id) }),
        project_id,
        verification_method_id: new_test.verification_method_id.and_then(|id| {
            if id == 0 {
                None
            } else {
                Some(id)
            }
        }),
    };

    let id = service.create(&user, my_new_verification).map_err(|e| {
        eprintln!("Error creating verification: {:?}", e);
        Redirect::to(new_test_path(
            &project_slug,
            Some("Failed to create verification"),
        ))
    })?;

    // Link requirements
    #[cfg(debug_assertions)]
    println!(
        "NewVerificationForm requirements: {:#?}",
        new_test.verification_req
    );
    for req in new_test.verification_req.iter() {
        let matrix_item = NewMatrixLink {
            req_id: *req,
            verification_id: id,
            // Use the route's project_id (from the URL) rather than the form field to
            // prevent a tampered form from creating cross-project links.
            project_id,
            triggering_version_id: None,
            triggering_user_id: None,
        };
        state
            .repo_write()
            .insert_new_matrix_item(&matrix_item)
            .map_err(|e| {
                use crate::repository::errors::RepoError;
                let msg = match e {
                    RepoError::CrossProjectViolation(ref detail) => {
                        format!("Cannot link requirement across projects: {}", detail)
                    }
                    _ => "Failed to link requirements".to_string(),
                };
                eprintln!("Error inserting matrix item: {:?}", e);
                Redirect::to(new_test_path(&project_slug, Some(&msg)))
            })?;
    }

    Ok(Redirect::to(test_detail_path(&project_slug, id)))
}

#[get("/<project_id>/verifications/edit/<test_id>")]
async fn get_edit_test(
    project_access: HtmlProjectAccess,
    project_id: String,
    test_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let repo = state.repo_read();

    let test = match repo.get_verification_by_id(test_id) {
        Ok(t) => t,
        Err(_) => return Err(Redirect::to(verifications_list_path(&project_slug))),
    };
    if test.project_id != project_id {
        return Err(Redirect::to(edit_test_path(
            &get_project_slug_by_id_pooled_safe(state, test.project_id),
            test_id,
        )));
    }

    let decorated = decorate_tests_cached(state, vec![test]);
    let test0 = &decorated[0];

    let linked_requirements = get_requirements_for_test_cached(state, test_id).unwrap_or_default();
    let linked_req_ids: Vec<i32> = linked_requirements.iter().map(|r| r.id).collect();

    let ctx = json!({
        "tests": test0,
        "test_status_id": test0.verification_status_id,
        "categories": repo.get_categories_by_project(project_id).unwrap_or_default(),
        "status": project_test_statuses(state.inner(), project_id),
        "parent": repo.get_verifications_by_project(project_id).unwrap_or_default(),
        "users": repo.get_users_all().unwrap_or_default(),
        "verification": repo.get_verification_methods_by_project(project_id).unwrap_or_default(),
        "linked_requirements": linked_requirements,
        "linked_req_ids": linked_req_ids,
        "requirements": repo.get_requirements_by_project(project_id).unwrap_or_default(),
        "project_id": project_id,
        "project_slug": project_slug,
        "user": user,
        "page_title": format!("Edit {} - Test", test0.reference_code)
    });

    #[cfg(debug_assertions)]
    println!("Tests: {:#}", ctx);

    Ok(Template::render("verifications/edit_verification", ctx))
}

/// Returns only the edit-panel HTML fragment (no layout). Used by the tests list page side panel.
#[get("/<project_id>/verifications/edit-panel/<test_id>")]
async fn get_edit_test_panel(
    project_access: HtmlProjectAccess,
    project_id: String,
    test_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    use serde_json::json;

    let project_slug = project_id;
    let project_id = project_access.project_id();
    let _user = project_access.into_user();
    let repo = state.repo_read();

    let test = match repo.get_verification_by_id(test_id) {
        Ok(t) => t,
        Err(_) => return Err(Redirect::to(verifications_list_path(&project_slug))),
    };
    if test.project_id != project_id {
        return Err(Redirect::to(edit_test_panel_path(
            &get_project_slug_by_id_pooled_safe(state, test.project_id),
            test_id,
        )));
    }

    let decorated = decorate_tests_cached(state, vec![test]);
    let test0 = &decorated[0];

    let linked_requirements = get_requirements_for_test_cached(state, test_id).unwrap_or_default();
    let linked_req_ids: Vec<i32> = linked_requirements.iter().map(|r| r.id).collect();

    let ctx = json!({
        "tests": test0,
        "test_status_id": test0.verification_status_id,
        "categories": repo.get_categories_by_project(project_id).unwrap_or_default(),
        "status": project_test_statuses(state.inner(), project_id),
        "parent": repo.get_verifications_by_project(project_id).unwrap_or_default(),
        "verification": repo.get_verification_methods_by_project(project_id).unwrap_or_default(),
        "linked_requirements": linked_requirements,
        "linked_req_ids": linked_req_ids,
        "requirements": repo.get_requirements_by_project(project_id).unwrap_or_default(),
        "project_id": project_id,
        "project_slug": project_slug,
    });

    Ok(Template::render("verifications/edit_panel", ctx))
}

#[post(
    "/<project_id>/verifications/edit/<test_id>",
    data = "<edit_test_form>"
)]
async fn post_edit_test(
    project_access: HtmlProjectAccess,
    project_id: String,
    test_id: i32,
    edit_test_form: Form<EditVerificationForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let service = VerificationService::new(state.inner());
    let to_list = || Redirect::to(verifications_list_path(&project_slug));
    if let Ok(existing) = service.get_by_id(test_id) {
        if existing.project_id != project_id {
            return Err(Redirect::to(edit_test_path(
                &get_project_slug_by_id_pooled_safe(state, existing.project_id),
                test_id,
            )));
        }
    }

    // Own the form to avoid cloning strings
    let f = edit_test_form.into_inner();

    let new_verification = NewVerification {
        id: Some(f.id),
        name: f.name,
        description: f.description,
        source: f.source,
        status_id: f.status_id,
        reference_code: f.reference_code,
        parent_id: f
            .parent_id
            .and_then(|id| if id == 0 { None } else { Some(id) }),
        project_id: f.project_id,
        verification_method_id: f.verification_method_id.and_then(|id| {
            if id == 0 {
                None
            } else {
                Some(id)
            }
        }),
    };

    service
        .update(&user, test_id, new_verification)
        .map_err(|e| {
            eprintln!("Error editing test: {e:?}");
            to_list()
        })?;

    state
        .repo_write()
        .update_verification_requirement_links(f.id, &f.linked_requirements)
        .map_err(|e| {
            eprintln!("Error updating test requirement links: {e:?}");
            to_list()
        })?;

    Ok(Redirect::to(test_detail_path(&project_slug, f.id)))
}

#[delete("/<project_id>/verifications/delete/<test_id>")]
async fn delete_test_route(
    project_access: HtmlProjectAccess,
    project_id: String,
    test_id: i32,
    state: &State<AppState>,
) -> Result<Redirect, rocket::http::Status> {
    use rocket::http::Status;

    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let service = VerificationService::new(state.inner());

    let test = service.get_by_id(test_id).map_err(|_| Status::NotFound)?;
    if test.project_id != project_id {
        return Ok(verifications_list_redirect(
            &get_project_slug_by_id_pooled_safe(state, test.project_id),
        ));
    }

    // Permission gate: only allow deletion of tests in Passed or Failed status, or if admin
    // Resolve the test's status title from the project's status list (not hardcoded IDs)
    let is_deletable = StatusService::new(state.inner())
        .get_verification_status(test.status_id)
        .ok()
        .and_then(|status| TestStatusEnum::from_title(&status.title))
        .map(|status| matches!(status, TestStatusEnum::Passed | TestStatusEnum::Failed))
        .unwrap_or(false);

    if !is_deletable && !user.is_admin {
        return Err(Status::Forbidden);
    }

    service.delete(&user, test_id).map_err(|e| match e {
        crate::repository::errors::RepoError::NotFound => Status::NotFound,
        _ => Status::InternalServerError,
    })?;

    Ok(Redirect::to(verifications_list_path(&project_slug)))
}

/// POST /p/<project_id>/verifications/update-status/<test_id> — inline status update (uses same session as page).
/// Accepts JSON body: { "status_id": 1 } for reliable parsing.
#[post(
    "/<project_id>/verifications/update-status/<test_id>",
    data = "<payload>"
)]
async fn update_test_status_route(
    project_access: HtmlProjectAccess,
    project_id: String,
    test_id: i32,
    payload: Json<UpdateTestStatusForm>,
    state: &State<AppState>,
) -> Result<Json<Value>, (rocket::http::Status, String)> {
    use rocket::http::Status;

    let _project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let service = VerificationService::new(state.inner());

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
    let updated = NewVerification {
        id: Some(test.id),
        reference_code: test.reference_code,
        name: test.name,
        description: test.description,
        source: test.source,
        status_id,
        parent_id: test.parent_id,
        project_id: test.project_id,
        verification_method_id: test.verification_method_id,
    };

    service.update(&user, test_id, updated).map_err(|e| {
        eprintln!("Error updating test status: {:?}", e);
        (Status::InternalServerError, "Update failed".to_string())
    })?;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[get("/<project_id>/requirements.xls")]
async fn get_requirements_xls(
    project_access: HtmlProjectAccess,
    project_id: String,
) -> Result<(ContentType, NamedFile), Redirect> {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    println!(
        "User [{} - id:{}] requested requirements export for project_id={}",
        user.username, user.id, project_id
    );

    excel::create_requirements_workbook(project_id).map_err(|e| {
        eprintln!("Error creating requirements workbook: {e:?}");
        Redirect::to(format!("/p/{project_slug}/requirements"))
    })?;
    let path_to_file = path::Path::new("target/requirements.xls");
    let file = NamedFile::open(&path_to_file).await.map_err(|e| {
        eprintln!("Error opening requirements export file: {e:?}");
        Redirect::to(format!("/p/{project_slug}/requirements"))
    })?;
    let content_type = ContentType::new(
        "application",
        "vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    );
    Ok((content_type, file))
}

#[get("/<project_id>/verifications.xls")]
async fn get_tests_xls(
    project_access: HtmlProjectAccess,
    project_id: String,
) -> Result<(ContentType, NamedFile), Redirect> {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    println!(
        "User [{} - id:{}] requested tests export for project_id={}",
        user.username, user.id, project_id
    );
    excel::create_tests_workbook(project_id).map_err(|e| {
        eprintln!("Error creating tests workbook: {e:?}");
        Redirect::to(verifications_list_path(&project_slug))
    })?;
    let path_to_file = path::Path::new("target/verifications.xls");
    let file = NamedFile::open(&path_to_file).await.map_err(|e| {
        eprintln!("Error opening tests export file: {e:?}");
        Redirect::to(verifications_list_path(&project_slug))
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
        show_verification_version,
        new_test,
        get_edit_test,
        get_edit_test_panel,
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
        RequirementStatus, Verification, VerificationMethod, VerificationStatus,
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
            slug: name.to_lowercase().replace(' ', "-"),
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
            is_system: false,
            tag_color: None,
        }
    }

    fn sample_test_status(id: i32, title: &str) -> VerificationStatus {
        VerificationStatus {
            id,
            title: title.to_string(),
            description: format!("{title} status"),
            tag: title.to_ascii_uppercase(),
            project_id: 1,
            is_system: false,
            tag_color: None,
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
            same_as_current: None,
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
            custom_fields: None,
        }
    }

    fn sample_test(id: i32, status: i32, name: &str) -> Verification {
        Verification {
            id,
            name: name.to_string(),
            description: format!("{name} description"),
            source: "Design Spec".into(),
            status_id: status,
            reference_code: format!("TEST-{id:03}"),
            parent_id: None,
            project_id: PRIMARY_PROJECT,
            verification_method_id: None,
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
        repo.verification_statuses
            .insert(1, sample_test_status(1, "Draft"));
        repo.verification_statuses
            .insert(2, sample_test_status(2, "Proposal"));
        repo.verification_statuses
            .insert(3, sample_test_status(3, "Active"));

        repo.categories.insert(1, sample_category(1, "Systems"));
        repo.verification_methods
            .insert(1, sample_verification(1, "Analysis"));
        repo.applicability.insert(1, sample_applicability(1, "All"));
        repo.requirements.insert(1, sample_requirement(1));

        repo
    }

    fn repo_with_tests() -> DieselRepoMock {
        let mut repo = base_repo();
        repo.verifications
            .insert(1, sample_test(1, 1, "Baseline Test"));
        repo.matrices.push(MatrixLink {
            req_id: 1,
            verification_id: 1,
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
        repo.verifications
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
        let response = get_with_session(&client, "/p/orbiter/verifications", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Baseline Test"));
    }

    #[rocket::async_test]
    async fn show_test_id_displays_details() {
        let client = test_client(repo_with_tests()).await;
        let response = get_with_session(&client, "/p/orbiter/verifications/show/1", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Baseline Test"));
        assert!(body.contains("description"));
    }

    #[rocket::async_test]
    async fn show_test_id_returns_error_when_missing() {
        let client = test_client(base_repo()).await;
        let response =
            get_with_session(&client, "/p/orbiter/verifications/show/42", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Test Not Found"));
    }

    #[rocket::async_test]
    async fn new_test_form_renders() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/orbiter/verifications/new", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("New Verification"));
        assert!(body.contains("Create Verification"));
    }

    #[rocket::async_test]
    async fn post_test_creates_new_entry() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/orbiter/verifications/new",
            concat!(
                "name=Thermal+Check&reference_code=TEST-002&description=Thermal+validation&",
                "source=Spec&status_id=1&parent_id=0&verification_req=1&project_id=1"
            ),
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), HttpStatus::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/orbiter/verifications/show/1")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let inner = repo.inner_repo();

        let test = inner.verifications.get(&1).expect("inserted test");
        assert_eq!(test.name, "Thermal Check");
        assert_eq!(test.status_id, 1);

        let links: Vec<_> = inner
            .matrices
            .iter()
            .filter(|m| m.verification_id == 1)
            .collect();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].req_id, 1);
    }

    #[rocket::async_test]
    async fn get_edit_test_renders_existing_data() {
        let client = test_client(repo_with_tests()).await;
        let response = get_with_session(&client, "/p/orbiter/verifications/edit/1", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Edit Verification"));
        assert!(body.contains("Baseline Test"));
    }

    #[rocket::async_test]
    async fn post_edit_test_updates_entry() {
        let client = test_client(repo_with_tests()).await;
        let response = post_form_with_session(
            &client,
            "/p/orbiter/verifications/edit/1",
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
            Some("/p/orbiter/verifications/show/1")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let inner = repo.inner_repo();

        let test = inner.verifications.get(&1).expect("existing test");
        assert_eq!(test.name, "Updated Test");
        assert_eq!(test.status_id, 2);

        let links: Vec<_> = inner
            .matrices
            .iter()
            .filter(|m| m.verification_id == 1)
            .collect();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].req_id, 1);
    }

    #[rocket::async_test]
    async fn delete_test_route_removes_draft() {
        let client = test_client(repo_with_tests()).await;
        let response =
            delete_with_session(&client, "/p/orbiter/verifications/delete/1", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::SeeOther);
        let location = response.headers().get_one("Location");
        assert!(location.is_some());
        assert!(location.unwrap().contains("/p/orbiter/verifications"));

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        assert!(repo.inner_repo().verifications.is_empty());
    }

    #[rocket::async_test]
    async fn delete_test_route_forbids_non_admin_when_status_high() {
        let client = test_client(repo_with_active_test()).await;
        let response =
            delete_with_session(&client, "/p/orbiter/verifications/delete/1", USER_ID).await;

        assert_eq!(response.status(), HttpStatus::Forbidden);
    }

    #[rocket::async_test]
    async fn show_tests_requires_membership_for_non_admin() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/orbiter/verifications", USER_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
    }
}
