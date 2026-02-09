//! HTML routes for immutable project baselines.

use super::helpers;
use super::prelude::*;
use crate::services::BaselineService;
use rocket::form::FromForm;

#[derive(Debug, FromForm)]
#[allow(dead_code)]
pub struct CreateBaselineForm {
    pub name: String,
    pub description: Option<String>,
}

#[get("/<project_id>/baselines")]
pub async fn show_baselines(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let projects = helpers::get_accessible_projects(state, &user);
    if !projects.iter().any(|p| p.id == project_id) {
        return Err(Redirect::to("/projects"));
    }
    let service = BaselineService::new(state.inner());
    let baselines = service.list_by_project(project_id).unwrap_or_default();
    let repo = state.repo_read();
    let created_by_names: std::collections::HashMap<i32, String> = baselines
        .iter()
        .map(|b| b.created_by)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .filter_map(|uid| repo.get_user_by_id(uid).ok().map(|u| (uid, u.name)))
        .collect();

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "baselines": baselines,
        "created_by_names": created_by_names,
        "page_title": "Baselines"
    });

    Ok(Template::render("baselines/baselines", ctx))
}

#[get("/<project_id>/baselines/new")]
pub async fn new_baseline_form(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let projects = helpers::get_accessible_projects(state, &user);
    if !projects.iter().any(|p| p.id == project_id) {
        return Err(Redirect::to("/projects"));
    }

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "page_title": "New Baseline"
    });

    Ok(Template::render("baselines/new_baseline", ctx))
}

#[post("/<project_id>/baselines/new", data = "<form>")]
pub async fn post_baseline(
    project_access: ProjectAccess,
    project_id: i32,
    form: Form<CreateBaselineForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();
    let projects = helpers::get_accessible_projects(state, &user);
    if !projects.iter().any(|p| p.id == project_id) {
        return Err(Redirect::to("/projects"));
    }

    let list_url = uri!("/p", show_baselines(project_id));
    let new_url = uri!("/p", new_baseline_form(project_id));

    let description = form.description.as_ref().and_then(|s| {
        if s.trim().is_empty() {
            None
        } else {
            Some(s.clone())
        }
    });
    let payload = crate::models::NewBaseline {
        name: form.name.clone(),
        description,
    };

    let service = BaselineService::new(state.inner());
    if let Err(e) = service.create_baseline(project_id, user.id, &payload) {
        eprintln!("create_baseline error: {:?}", e);
        return Ok(Redirect::to(new_url));
    }

    Ok(Redirect::to(list_url))
}

#[get("/<project_id>/baselines/<baseline_id>", rank = 2)]
pub async fn show_baseline(
    project_access: ProjectAccess,
    project_id: i32,
    baseline_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let projects = helpers::get_accessible_projects(state, &user);
    if !projects.iter().any(|p| p.id == project_id) {
        return Err(Redirect::to("/projects"));
    }

    let service = BaselineService::new(state.inner());
    let baseline = service
        .get_by_id(baseline_id)
        .map_err(|_| Redirect::to(uri!("/p", show_baselines(project_id))))?;
    if baseline.project_id != project_id {
        return Err(Redirect::to(uri!("/p", show_baselines(project_id))));
    }

    let requirements = service.get_requirements(baseline_id).unwrap_or_default();
    let traceability = service.get_traceability(baseline_id).unwrap_or_default();
    let version_by_req: std::collections::HashMap<i32, i32> = requirements
        .iter()
        .filter_map(|r| r.current_version_id.map(|vid| (r.id, vid)))
        .collect();
    let requirement_reference: std::collections::HashMap<i32, String> = requirements
        .iter()
        .map(|r| (r.id, r.reference_code.clone()))
        .collect();

    let repo = state.repo_read();
    let tests = repo
        .get_tests_by_project(project_id)
        .unwrap_or_default();
    let test_reference: std::collections::HashMap<i32, String> = tests
        .iter()
        .map(|t| (t.id, t.reference_code.clone()))
        .collect();

    // One entry per (requirement, test) link — do not collapse to one test per requirement
    let traceability_links: Vec<serde_json::Value> = traceability
        .iter()
        .map(|t| {
            let req_ref = requirement_reference
                .get(&t.requirement_id)
                .cloned()
                .unwrap_or_else(|| format!("#{}", t.requirement_id));
            let tst_ref = test_reference
                .get(&t.test_id)
                .cloned()
                .unwrap_or_else(|| format!("#{}", t.test_id));
            serde_json::json!({
                "requirement_id": t.requirement_id,
                "test_id": t.test_id,
                "requirement_reference": req_ref,
                "test_reference": tst_ref,
                "version_id": version_by_req.get(&t.requirement_id).copied(),
            })
        })
        .collect();

    let created_by_name = repo
        .get_user_by_id(baseline.created_by)
        .ok()
        .map(|u| u.name);

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "baseline": baseline,
        "requirements": requirements,
        "traceability": traceability_links,
        "created_by_name": created_by_name,
        "page_title": format!("Baseline: {}", baseline.name)
    });

    Ok(Template::render("baselines/baseline", ctx))
}

pub fn routes() -> Vec<Route> {
    routes![
        show_baselines,
        new_baseline_form,
        post_baseline,
        show_baseline,
    ]
}
