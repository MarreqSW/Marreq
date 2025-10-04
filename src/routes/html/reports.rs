use super::helpers::*;
use super::prelude::*;
use crate::app::DieselCachedRepo;
use crate::models::{Category, Requirement, Test, User};
use std::collections::HashMap;

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

fn get_details(
    project_id: i32,
    repo: &DieselCachedRepo,
) -> (Vec<Requirement>, Vec<Test>, Vec<Category>) {
    (
        repo.get_requirements_by_project(project_id)
            .unwrap_or_default(),
        repo.get_tests_by_project(project_id).unwrap_or_default(),
        repo.get_categories_by_project(project_id)
            .unwrap_or_default(),
    )
}

fn compute_metrics(state: &State<AppState>, project_id: i32) -> (Metrics, String) {
    let repo = state.repo_read();

    let (requirements, tests, categories) = get_details(project_id, &repo);
    let users_len = repo.get_users_all().unwrap_or_default().len();
    let all_statuses = repo.get_status_all().unwrap_or_default();

    // group helpers (i32 counts)
    let requirements_by_status = requirements.iter().fold(HashMap::new(), |mut acc, req| {
        let status = get_status_name_by_id_cached(state, req.req_current_status);
        *acc.entry(status).or_insert(0) += 1i32;
        acc
    });

    let tests_by_status = tests.iter().fold(HashMap::new(), |mut acc, t| {
        let status = get_status_name_by_id_cached(state, t.test_status);
        *acc.entry(status).or_insert(0) += 1i32;
        acc
    });

    let requirements_by_category = requirements.iter().fold(HashMap::new(), |mut acc, req| {
        let cat = get_category_by_id_cached(state, req.req_category);
        *acc.entry(cat.cat_title.clone()).or_insert(0) += 1i32;
        acc
    });

    // coverage
    let mut covered = 0usize;
    let mut total_links = 0usize;
    for req in &requirements {
        let links = get_requirements_for_test_cached(state, req.req_id).unwrap_or_default();
        if !links.is_empty() {
            covered += 1;
        }
        total_links += links.len();
    }

    let total_requirements = requirements.len();
    let coverage_percentage = if total_requirements > 0 {
        round1((covered as f64 / total_requirements as f64) * 100.0)
    } else {
        0.0
    };

    let avg_tests_per_requirement = if total_requirements > 0 {
        round1(total_links as f64 / total_requirements as f64)
    } else {
        0.0
    };

    let recent_requirements = requirements.len();
    let recent_tests = tests.len();

    let selected_project_name = get_project_by_id_pooled_safe(state, project_id).project_name;

    let metrics = Metrics {
        users_len,
        total_requirements,
        total_tests: tests.len(),
        statuses: all_statuses,
        total_categories: categories.len(),
        requirements_by_status,
        tests_by_status,
        requirements_by_category,
        covered_requirements: covered,
        total_links,
        coverage_percentage,
        avg_tests_per_requirement,
        recent_requirements,
        recent_tests,
        categories,
    };

    (metrics, selected_project_name)
}

// --- routes (unchanged, but now types match) --------------------------------

#[get("/<project_id>/reports")]
async fn show_reports(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user: User = project_access.into_user();

    let (m, project_name) = compute_metrics(state, project_id);

    let ctx = serde_json::json!({
        "user": user,
        "selected_project_id": project_id,
        "selected_project_name": project_name,
        "metrics": {
            "total_requirements": m.total_requirements,
            "total_tests": m.total_tests,
            "total_categories": m.total_categories,
            "total_users": m.users_len,
            "coverage_percentage": m.coverage_percentage,
            "avg_tests_per_requirement": m.avg_tests_per_requirement,
            "covered_requirements": m.covered_requirements,
            "total_links": m.total_links,
            "recent_requirements": m.recent_requirements,
            "recent_tests": m.recent_tests
        },
        "requirements_by_status": m.requirements_by_status,
        "tests_by_status": m.tests_by_status,
        "requirements_by_category": m.requirements_by_category,
        "all_statuses": m.statuses,
        "all_categories": m.categories
    });

    Ok(Template::render("reports", ctx))
}

#[get("/<project_id>/reports/pdf")]
async fn generate_pdf_report(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<(rocket::http::ContentType, Vec<u8>), Redirect> {
    let _user: User = project_access.into_user();

    let (m, _project_name) = compute_metrics(state, project_id);

    match generate_pdf_report_data(&m) {
        Ok(pdf_bytes) => {
            let ct = rocket::http::ContentType::new("application", "pdf");
            Ok((ct, pdf_bytes))
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("PDF generation failed: {:?}", _e);
            let ct = rocket::http::ContentType::new("text", "html");
            Ok((ct, generate_pdf_content(&m).into_bytes()))
        }
    }
}

pub fn routes() -> Vec<Route> {
    routes![show_reports, generate_pdf_report,]
}
