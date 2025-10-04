use super::helpers::*;
use super::prelude::*;
use crate::app::DieselCachedRepo;
use crate::models::{Category, Requirement, Test, User};

fn get_details(project_id: i32, repo: &DieselCachedRepo)
    -> (Vec<Requirement>, Vec<Test>, Vec<Category>)
{
    (repo
        .get_requirements_by_project(project_id)
        .unwrap_or_default(),
    repo
        .get_tests_by_project(project_id)
        .unwrap_or_default(),
    repo
        .get_categories_by_project(project_id)
        .unwrap_or_default())
}

#[get("/<project_id>/reports")]
async fn show_reports(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user: User = project_access.into_user();

    let (all_requirements, all_tests, all_categories) = get_details(project_id, &state.repo_read());

    let all_users = state.repo_read().get_users_all().unwrap_or_default();
    let all_statuses = state.repo_read().get_status_all().unwrap_or_default();

    // Calculate metrics
    let total_requirements = all_requirements.len();
    let total_tests = all_tests.len();
    let total_categories = all_categories.len();
    let total_users = all_users.len();

    // Requirements by status
    let mut requirements_by_status = std::collections::HashMap::new();
    for req in &all_requirements {
        let status_name = get_status_name_by_id_cached(state, req.req_current_status);
        *requirements_by_status.entry(status_name).or_insert(0) += 1;
    }

    // Tests by status
    let mut tests_by_status = std::collections::HashMap::new();
    for test in &all_tests {
        let status_name = get_status_name_by_id_cached(state, test.test_status);
        *tests_by_status.entry(status_name).or_insert(0) += 1;
    }

    // Requirements by category
    let mut requirements_by_category = std::collections::HashMap::new();
    for req in &all_requirements {
        let category = get_category_by_id_cached(state, req.req_category);
        let category_name = category.cat_title;
        *requirements_by_category.entry(category_name).or_insert(0) += 1;
    }

    // Coverage metrics
    let mut covered_requirements = 0;
    let mut total_links = 0;
    for req in &all_requirements {
        let links = get_requirements_for_test_cached(state, req.req_id).unwrap_or_default();
        if !links.is_empty() {
            covered_requirements += 1;
        }
        total_links += links.len();
    }

    let coverage_percentage = if total_requirements > 0 {
        ((covered_requirements as f64 / total_requirements as f64) * 100.0 * 10.0).round() / 10.0
    } else {
        0.0
    };

    let avg_tests_per_requirement = if total_requirements > 0 {
        ((total_links as f64 / total_requirements as f64) * 10.0).round() / 10.0
    } else {
        0.0
    };

    // Recent activity (last 30 days)
    let now = chrono::Utc::now();
    let _thirty_days_ago = now - chrono::Duration::days(30);

    let mut recent_requirements = 0;
    let mut recent_tests = 0;

    for _req in &all_requirements {
        // For now, we'll use a placeholder since creation_date might not be available
        recent_requirements += 1; // Placeholder
    }

    for _test in &all_tests {
        // Assuming test has creation date - you might need to add this field
        // For now, we'll use a placeholder
        recent_tests += 1; // Placeholder
    }

    // Get selected project name for display
    let selected_project_name = get_project_by_id_pooled_safe(state, project_id).project_name;

    let ctx = json!({
        "user": user,
        "selected_project_name": selected_project_name,
        "metrics": {
            "total_requirements": total_requirements,
            "total_tests": total_tests,
            "total_categories": total_categories,
            "total_users": total_users,
            "coverage_percentage": coverage_percentage,
            "avg_tests_per_requirement": avg_tests_per_requirement,
            "covered_requirements": covered_requirements,
            "total_links": total_links,
            "recent_requirements": recent_requirements,
            "recent_tests": recent_tests
        },
        "requirements_by_status": requirements_by_status,
        "tests_by_status": tests_by_status,
        "requirements_by_category": requirements_by_category,
        "all_statuses": all_statuses,
        "all_categories": all_categories
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

    // Get project-specific data for metrics
    let (all_requirements, all_tests, all_categories) = get_details(project_id, &state.repo_read());

    let all_users = state.repo_read().get_users_all().unwrap_or_default();
    let _all_statuses = state.repo_read().get_status_all().unwrap_or_default();

    // Calculate the same metrics
    let total_requirements = all_requirements.len();
    let total_tests = all_tests.len();
    let total_categories = all_categories.len();
    let total_users = all_users.len();

    // Requirements by status
    let mut requirements_by_status = std::collections::HashMap::new();
    for req in &all_requirements {
        let status_name = get_status_name_by_id_cached(state, req.req_current_status);
        *requirements_by_status.entry(status_name).or_insert(0) += 1;
    }

    // Tests by status
    let mut tests_by_status = std::collections::HashMap::new();
    for test in &all_tests {
        let status_name = get_status_name_by_id_cached(state, test.test_status);
        *tests_by_status.entry(status_name).or_insert(0) += 1;
    }

    // Requirements by category
    let mut requirements_by_category = std::collections::HashMap::new();
    for req in &all_requirements {
        let category = get_category_by_id_cached(state, req.req_category);
        let category_name = category.cat_title;
        *requirements_by_category.entry(category_name).or_insert(0) += 1;
    }

    // Coverage metrics
    let mut covered_requirements = 0;
    let mut total_links = 0;
    for req in &all_requirements {
        let links = get_requirements_for_test_cached(state, req.req_id).unwrap_or_default();
        if !links.is_empty() {
            covered_requirements += 1;
        }
        total_links += links.len();
    }

    let coverage_percentage = if total_requirements > 0 {
        ((covered_requirements as f64 / total_requirements as f64) * 100.0 * 10.0).round() / 10.0
    } else {
        0.0
    };

    let avg_tests_per_requirement = if total_requirements > 0 {
        ((total_links as f64 / total_requirements as f64) * 10.0).round() / 10.0
    } else {
        0.0
    };

    // Generate HTML content
    let html_content = generate_pdf_content(
        total_requirements,
        total_tests,
        total_categories,
        total_users,
        coverage_percentage,
        avg_tests_per_requirement,
        covered_requirements,
        total_links,
        requirements_by_status.clone(),
        tests_by_status.clone(),
        requirements_by_category.clone(),
    );

    // Generate PDF using the new PDF generation function
    match generate_pdf_report_data(
        total_requirements,
        total_tests,
        total_categories,
        total_users,
        coverage_percentage,
        avg_tests_per_requirement,
        covered_requirements,
        total_links,
        requirements_by_status,
        tests_by_status,
        requirements_by_category,
    ) {
        Ok(pdf_bytes) => {
            let content_type = rocket::http::ContentType::new("application", "pdf");
            Ok((content_type, pdf_bytes))
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("PDF generation failed: {:?}", _e);
            // Fallback to HTML if PDF generation fails
            let content_type = rocket::http::ContentType::new("text", "html");
            Ok((content_type, html_content.into_bytes()))
        }
    }
}


pub fn routes() -> Vec<Route> {
    routes![
        show_reports,
        generate_pdf_report,
    ]
}


/*
#[get("/test-pdf")]
pub fn test_pdf_generation() -> Result<(rocket::http::ContentType, Vec<u8>), rocket::http::Status> {
    // Test data
    let total_requirements = 150;
    let total_tests = 120;
    let total_categories = 8;
    let total_users = 12;
    let coverage_percentage = 85.5;
    let avg_tests_per_requirement = 1.2;
    let covered_requirements = 128;
    let total_links = 180;

    let mut requirements_by_status = std::collections::HashMap::new();
    requirements_by_status.insert("Active".to_string(), 100);
    requirements_by_status.insert("Draft".to_string(), 30);
    requirements_by_status.insert("Deprecated".to_string(), 20);

    let mut tests_by_status = std::collections::HashMap::new();
    tests_by_status.insert("Passed".to_string(), 80);
    tests_by_status.insert("Failed".to_string(), 15);
    tests_by_status.insert("Pending".to_string(), 25);

    let mut requirements_by_category = std::collections::HashMap::new();
    requirements_by_category.insert("Functional".to_string(), 80);
    requirements_by_category.insert("Non-Functional".to_string(), 40);
    requirements_by_category.insert("Interface".to_string(), 30);

    // Generate PDF using the PDF generation function
    match generate_pdf_report_data(
        total_requirements,
        total_tests,
        total_categories,
        total_users,
        coverage_percentage,
        avg_tests_per_requirement,
        covered_requirements,
        total_links,
        requirements_by_status,
        tests_by_status,
        requirements_by_category,
    ) {
        Ok(pdf_bytes) => {
            let content_type = rocket::http::ContentType::new("application", "pdf");
            Ok((content_type, pdf_bytes))
        }
        Err(e) => {
            eprintln!("PDF generation failed: {:?}", e);
            Err(rocket::http::Status::InternalServerError)
        }
    }
}
*/