use super::helpers::*;
use super::prelude::*;
use crate::app::DieselCachedRepo;
use crate::models::{Category, Requirement, TestCase, User};
use std::collections::HashMap;

fn round1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

fn get_details(
    project_id: i32,
    repo: &DieselCachedRepo,
) -> (Vec<Requirement>, Vec<TestCase>, Vec<Category>) {
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
    let all_statuses = repo.get_requirement_status_all().unwrap_or_default();

    // group helpers (i32 counts)
    let requirements_by_status = requirements.iter().fold(HashMap::new(), |mut acc, req| {
        let status = get_status_name_by_id_cached(state, req.status_id);
        *acc.entry(status).or_insert(0) += 1i32;
        acc
    });

    let tests_by_status = tests.iter().fold(HashMap::new(), |mut acc, t| {
        let status = get_status_name_by_id_cached(state, t.status_id);
        *acc.entry(status).or_insert(0) += 1i32;
        acc
    });

    let requirements_by_category = requirements.iter().fold(HashMap::new(), |mut acc, req| {
        let cat = get_category_by_id_cached(state, req.category_id);
        *acc.entry(cat.title.clone()).or_insert(0) += 1i32;
        acc
    });

    // coverage
    let mut covered = 0usize;
    let mut total_links = 0usize;
    for req in &requirements {
        let links = get_requirements_for_test_cached(state, req.id).unwrap_or_default();
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

    let selected_project_name = get_project_by_id_pooled_safe(state, project_id).name;

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

    let (m, name) = compute_metrics(state, project_id);

    let ctx = serde_json::json!({
        "user": user,
        "selected_project_id": project_id,
        "selected_project_name": name,
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
        "all_categories": m.categories,
        "page_title": format!("{} - Reports", name)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Category, MatrixLink, Project, ProjectMember, Requirement, RequirementStatus, TestCase,
    };
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::routes::html::project::test_helpers::{
        client_with_routes, get_with_session, timestamp,
    };
    use crate::status_enums::ProjectStatus;
    use rocket::http::{ContentType, Status as HttpStatus};
    use rocket::local::asynchronous::Client;

    const ADMIN_ID: i32 = 1;
    const PROJECT_ID: i32 = 1;

    fn sample_project() -> Project {
        Project {
            id: PROJECT_ID,
            name: "Orbiter".to_string(),
            description: Some("Orbiter project".to_string()),
            creation_date: Some(timestamp()),
            update_date: Some(timestamp()),
            status: ProjectStatus::Active,
            owner_id: Some(ADMIN_ID),
        }
    }

    fn sample_category() -> Category {
        Category {
            id: 1,
            title: "Systems".to_string(),
            description: "Core systems".to_string(),
            tag: "systems".to_string(),
            project_id: PROJECT_ID,
        }
    }

    fn sample_status(id: i32, title: &str) -> RequirementStatus {
        RequirementStatus {
            id,
            title: title.to_string(),
            description: format!("{title} status"),
            tag: title.chars().take(3).collect(),
            project_id: 1,
        }
    }

    fn sample_requirement(id: i32) -> Requirement {
        Requirement {
            id,
            current_version_id: None,
            title: format!("Requirement {id}"),
            description: "Test requirement".to_string(),
            status_id: 1,
            author_id: ADMIN_ID,
            reviewer_id: ADMIN_ID,
            reference_code: format!("REQ-{:03}", id),
            category_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            applicability_id: 1,
            justification: Some("For testing".to_string()),
            project_id: PROJECT_ID,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
        }
    }

    fn sample_test(id: i32, status_id: i32, name: &str) -> TestCase {
        TestCase {
            id,
            name: name.to_string(),
            description: "Validation test".to_string(),
            source: "Spec".to_string(),
            status_id,
            reference_code: format!("TEST-{id:03}"),
            parent_id: None,
            project_id: PROJECT_ID,
        }
    }

    fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();

        let mut admin = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
        admin.is_admin = true;
        repo.users.insert(ADMIN_ID, admin);

        repo.projects.insert(PROJECT_ID, sample_project());
        repo.categories.insert(1, sample_category());
        repo.requirement_statuses
            .insert(1, sample_status(1, "Draft"));
        repo.requirement_statuses
            .insert(2, sample_status(2, "In Review"));
        repo.requirements.insert(1, sample_requirement(1));
        repo.tests.insert(1, sample_test(1, 1, "System Validation"));
        repo.matrices.push(MatrixLink {
            req_id: 1,
            test_id: 1,
            creation_date: timestamp(),
            project_id: PROJECT_ID,
            suspect: false,
            suspect_at: None,
            suspect_reason: None,
            cleared_by: None,
            cleared_at: None,
            triggering_version_id: None,
            triggering_user_id: None,
        });
        repo.project_members.push(ProjectMember {
            project_id: PROJECT_ID,
            user_id: ADMIN_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        repo
    }

    async fn test_client(repo: DieselRepoMock) -> Client {
        client_with_routes(repo, routes![show_reports, generate_pdf_report]).await
    }

    #[rocket::async_test]
    async fn show_reports_renders_metrics() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/reports", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Project Reports & Analytics"));
        assert!(body.contains("Orbiter"));
        assert!(body.contains("1 out of 1"));
        assert!(body.contains("Systems"));
        assert!(body.contains("Draft"));
    }

    #[rocket::async_test]
    async fn generate_pdf_report_returns_pdf_or_fallback_html() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/reports/pdf", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let content_type = response.content_type().expect("content type to be set");

        if content_type == ContentType::PDF {
            let body = response.into_bytes().await.expect("pdf bytes");
            assert!(!body.is_empty());
        } else {
            assert_eq!(content_type, ContentType::new("text", "html"));
            let body = response.into_string().await.expect("html body");
            assert!(body.contains("ReqMan Project Report"));
        }
    }
}
