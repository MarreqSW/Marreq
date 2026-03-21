// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! HTML routes for immutable project baselines.

use super::helpers;
use super::prelude::*;
use crate::routes::html::helpers::get_project_slug_by_id_pooled_safe;
use crate::services::BaselineService;
use rocket::form::FromForm;

#[derive(Debug, FromForm)]
#[allow(dead_code)]
pub struct CreateBaselineForm {
    pub name: String,
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_baseline_form_struct_exists() {
        let form = CreateBaselineForm {
            name: "v1.0".to_string(),
            description: Some("First release".to_string()),
        };
        assert_eq!(form.name, "v1.0");
        assert_eq!(form.description.as_deref(), Some("First release"));
    }

    #[test]
    fn create_baseline_form_description_optional() {
        let form = CreateBaselineForm {
            name: "v1.0".to_string(),
            description: None,
        };
        assert!(form.description.is_none());
    }
}

#[get("/<namespace>/<project_id>/baselines")]
pub async fn show_baselines(
    project_access: HtmlProjectAccess,
    namespace: String,
    project_id: String,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let project_slug = project_access.project_route_slug().to_string();
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let projects: Vec<_> = helpers::get_accessible_projects(state, &user)
        .iter()
        .map(|project| helpers::project_to_template_value(state, project))
        .collect();
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
        "selected_project_slug": project_slug,
        "baselines": baselines,
        "created_by_names": created_by_names,
        "page_title": "Baselines"
    });

    Ok(Template::render("baselines/baselines", ctx))
}

#[get("/<namespace>/<project_id>/baselines/new")]
pub async fn new_baseline_form(
    project_access: HtmlProjectAccess,
    namespace: String,
    project_id: String,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let project_slug = project_access.project_route_slug().to_string();
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let projects: Vec<_> = helpers::get_accessible_projects(state, &user)
        .iter()
        .map(|project| helpers::project_to_template_value(state, project))
        .collect();

    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": project_id,
        "selected_project_slug": project_slug,
        "page_title": "New Baseline"
    });

    Ok(Template::render("baselines/new_baseline", ctx))
}

#[post("/<namespace>/<project_id>/baselines/new", data = "<form>")]
pub async fn post_baseline(
    project_access: HtmlProjectAccess,
    namespace: String,
    project_id: String,
    form: Form<CreateBaselineForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let project_slug = project_access.project_route_slug().to_string();
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let projects: Vec<_> = helpers::get_accessible_projects(state, &user)
        .iter()
        .map(|project| helpers::project_to_template_value(state, project))
        .collect();

    let list_url = format!("/{project_slug}/baselines");
    let new_url = format!("/{project_slug}/baselines/new");

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

#[get("/<namespace>/<project_id>/baselines/<baseline_id>", rank = 2)]
pub async fn show_baseline(
    project_access: HtmlProjectAccess,
    namespace: String,
    project_id: String,
    baseline_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let project_slug = project_access.project_route_slug().to_string();
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    let projects: Vec<_> = helpers::get_accessible_projects(state, &user)
        .iter()
        .map(|project| helpers::project_to_template_value(state, project))
        .collect();

    let service = BaselineService::new(state.inner());
    let baseline = service
        .get_by_id(baseline_id)
        .map_err(|_| Redirect::to(format!("/{project_slug}/baselines")))?;
    if baseline.project_id != project_id {
        let baseline_project_slug = get_project_slug_by_id_pooled_safe(state, baseline.project_id);
        return Err(Redirect::to(format!("/{baseline_project_slug}/baselines")));
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

    let snapshot_verifications = service.get_verifications(baseline_id).unwrap_or_default();
    let test_reference: std::collections::HashMap<i32, String> =
        if snapshot_verifications.is_empty() {
            // Old baseline created before verification snapshot: fall back to current project verifications
            let repo = state.repo_read();
            let tests = repo
                .get_verifications_by_project(project_id)
                .unwrap_or_default();
            tests
                .iter()
                .map(|t| (t.id, t.reference_code.clone()))
                .collect()
        } else {
            snapshot_verifications
                .iter()
                .map(|v| (v.verification_id, v.reference_code.clone()))
                .collect()
        };

    let repo = state.repo_read();

    // One entry per (requirement, test) link — do not collapse to one test per requirement
    let traceability_links: Vec<serde_json::Value> = traceability
        .iter()
        .map(|t| {
            let req_ref = requirement_reference
                .get(&t.requirement_id)
                .cloned()
                .unwrap_or_else(|| format!("#{}", t.requirement_id));
            let tst_ref = test_reference
                .get(&t.verification_id)
                .cloned()
                .unwrap_or_else(|| format!("#{}", t.verification_id));
            serde_json::json!({
                "requirement_id": t.requirement_id,
                "verification_id": t.verification_id,
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
        "selected_project_slug": project_slug,
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
