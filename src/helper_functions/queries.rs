use crate::models::*;
use crate::repository::errors::RepoError;
use crate::repository::{
    DieselRepo, LookupRepository, MatrixRepository, ProjectsRepository, RequirementsRepository,
    TestsRepository, UserRepository,
};

pub type VerificationData = crate::models::Verification;

pub fn get_status_all() -> Result<Vec<Status>, String> {
    DieselRepo::new()
        .get_status_all()
        .map_err(|e| e.to_string())
}

pub fn get_categories_all() -> Result<Vec<Category>, String> {
    DieselRepo::new()
        .get_categories_all()
        .map_err(|e| e.to_string())
}

pub fn get_applicability_all() -> Result<Vec<Applicability>, String> {
    DieselRepo::new()
        .get_applicability_all()
        .map_err(|e| e.to_string())
}

pub fn get_applicability_by_id(id: i32) -> Applicability {
    DieselRepo::new()
        .get_applicability_by_id(id)
        .unwrap_or_else(|_| Applicability {
            app_id: id,
            app_title: format!("Unknown Applicability ({})", id),
            app_description: "Applicability not found".to_string(),
            app_tag: "unknown".to_string(),
            project_id: 1,
        })
}

pub fn get_applicability_by_id_safe(id: i32, target_project_id: i32) -> Applicability {
    DieselRepo::new()
        .get_applicability_by_id(id)
        .unwrap_or_else(|_| Applicability {
            app_id: id,
            app_title: format!("Unknown Applicability ({})", id),
            app_description: "Applicability not found".to_string(),
            app_tag: "unknown".to_string(),
            project_id: target_project_id,
        })
}

pub fn get_category_by_id(id: i32) -> Category {
    DieselRepo::new()
        .get_category_by_id(id)
        .unwrap_or_else(|_| Category {
            cat_id: id,
            cat_title: format!("Unknown Category ({})", id),
            cat_description: "Category not found".to_string(),
            cat_tag: "unknown".to_string(),
            project_id: 1,
        })
}

pub fn get_category_by_id_safe(id: i32, target_project_id: i32) -> Category {
    DieselRepo::new()
        .get_category_by_id(id)
        .unwrap_or_else(|_| Category {
            cat_id: id,
            cat_title: format!("Unknown Category ({})", id),
            cat_description: "Category not found".to_string(),
            cat_tag: "unknown".to_string(),
            project_id: target_project_id,
        })
}

pub fn get_user_by_id(id: i32) -> User {
    DieselRepo::new()
        .get_user_by_id(id)
        .expect("Error reading table Users")
}

pub fn get_user_by_username(uname: &str) -> Result<Option<User>, RepoError> {
    DieselRepo::new().get_user_by_username(uname)
}

pub fn get_status_by_id(id: i32) -> Status {
    DieselRepo::new()
        .get_status_by_id(id)
        .expect("Error reading table Status")
}

pub fn get_verification_all() -> Result<Vec<VerificationData>, String> {
    DieselRepo::new()
        .get_verification_all()
        .map_err(|e| e.to_string())
}

pub fn get_verification_by_project(project_id: i32) -> Result<Vec<VerificationData>, String> {
    DieselRepo::new()
        .get_verification_by_project(project_id)
        .map_err(|e| e.to_string())
}

pub fn get_verification_by_id(id: i32) -> VerificationData {
    DieselRepo::new()
        .get_verification_by_id(id)
        .unwrap_or_else(|_| VerificationData {
            verification_id: id,
            verification_name: format!("Unknown Verification ({})", id),
            verification_description: "Verification not found".to_string(),
            project_id: 1,
        })
}

pub fn get_verification_by_id_safe(id: i32, target_project_id: i32) -> VerificationData {
    DieselRepo::new()
        .get_verification_by_id(id)
        .unwrap_or_else(|_| VerificationData {
            verification_id: id,
            verification_name: format!("Unknown Verification ({})", id),
            verification_description: "Verification not found".to_string(),
            project_id: target_project_id,
        })
}

pub fn get_status_name_by_id(id: i32) -> String {
    get_status_by_id(id).st_title
}

pub fn get_requirement_by_id(id: i32) -> Requirement {
    DieselRepo::new()
        .get_requirement_by_id(id)
        .unwrap_or_else(|_| Requirement {
            req_id: id,
            req_title: format!("Unknown Requirement ({})", id),
            req_description: "Requirement not found".to_string(),
            req_verification: 1,
            req_current_status: 1,
            req_author: 1,
            req_reviewer: 1,
            req_link: "".to_string(),
            req_reference: format!("REQ-UNK-{}", id),
            req_category: 1,
            req_parent: 0,
            req_creation_date: chrono::Utc::now().naive_utc(),
            req_update_date: chrono::Utc::now().naive_utc(),
            req_deadline_date: chrono::Utc::now().naive_utc(),
            req_applicability: 1,
            req_justification: None,
            project_id: 1,
        })
}

pub fn get_requirement_by_id_safe(id: i32) -> Result<Requirement, String> {
    DieselRepo::new()
        .get_requirement_by_id(id)
        .map_err(|e| match e {
            RepoError::NotFound => format!("Requirement with ID {} not found", id),
            _ => e.to_string(),
        })
}

pub fn get_requirement_title_by_id(id: i32) -> String {
    match get_requirement_by_id_safe(id) {
        Ok(req) => req.req_title,
        Err(_) => "[Requirement Not Found]".to_string(),
    }
}

pub fn get_requirements_all() -> Result<Vec<Requirement>, String> {
    DieselRepo::new()
        .get_requirements_all()
        .map_err(|e| e.to_string())
}

pub fn get_tests_all() -> Result<Vec<Test>, String> {
    DieselRepo::new().get_tests_all().map_err(|e| e.to_string())
}

pub fn get_users_all() -> Result<Vec<User>, String> {
    DieselRepo::new().get_users_all().map_err(|e| e.to_string())
}

pub fn get_test_by_id(id: i32) -> Test {
    DieselRepo::new()
        .get_test_by_id(id)
        .expect("Error reading table Tests")
}

pub fn get_test_by_id_safe(id: i32) -> Result<Test, String> {
    DieselRepo::new().get_test_by_id(id).map_err(|e| match e {
        RepoError::NotFound => format!("Test with ID {} not found", id),
        _ => e.to_string(),
    })
}

pub fn get_test_status_by_id(id: i32) -> String {
    let repo = DieselRepo::new();
    let ts = match repo.get_test_by_id(id) {
        Ok(test) => test,
        Err(_) => return "[Test Not Found]".to_string(),
    };
    repo.get_status_by_id(ts.test_status)
        .map(|status| status.st_title)
        .unwrap_or_else(|_| "[Status Not Found]".to_string())
}

pub fn get_requirements_for_test(test_id: i32) -> Result<Vec<Requirement>, String> {
    DieselRepo::new()
        .get_requirements_for_test(test_id)
        .map_err(|e| e.to_string())
}

pub fn get_projects_all() -> Result<Vec<Project>, String> {
    DieselRepo::new()
        .get_projects_all()
        .map_err(|e| e.to_string())
}

pub fn get_project_by_id(project_id_param: i32) -> Project {
    DieselRepo::new()
        .get_project_by_id(project_id_param)
        .expect("Error loading project")
}

pub fn get_projects_for_nav() -> Result<Vec<Project>, String> {
    get_projects_all()
}

pub fn get_requirements_by_project(_project_id: i32) -> Result<Vec<Requirement>, String> {
    DieselRepo::new()
        .get_requirements_by_project(_project_id)
        .map_err(|e| e.to_string())
}

pub fn get_tests_by_project(_project_id: i32) -> Result<Vec<Test>, String> {
    DieselRepo::new()
        .get_tests_by_project(_project_id)
        .map_err(|e| e.to_string())
}

pub fn get_categories_by_project(_project_id: i32) -> Result<Vec<Category>, String> {
    DieselRepo::new()
        .get_categories_by_project(_project_id)
        .map_err(|e| e.to_string())
}

pub fn get_applicability_by_project(_project_id: i32) -> Result<Vec<Applicability>, String> {
    DieselRepo::new()
        .get_applicability_by_project(_project_id)
        .map_err(|e| e.to_string())
}

pub fn get_matrix_by_project(_project_id: i32) -> Result<Vec<Matrix>, String> {
    DieselRepo::new()
        .get_matrix_by_project(_project_id)
        .map_err(|e| e.to_string())
}
