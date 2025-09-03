use crate::models::*;
use crate::repository::{errors::RepoError, Repository, DieselRepo};

/// Decorate requirements using the default Diesel repository.
pub fn decorate_requirements(reqs: Vec<Requirement>) -> Vec<DecoratedRequirement> {
    let repo = DieselRepo::new();
    decorate_requirements_impl(&repo, reqs)
}

/// Decorate tests using the default Diesel repository.
pub fn decorate_tests(tests: Vec<Test>) -> Vec<DecoratedTest> {
    let repo = DieselRepo::new();
    decorate_tests_impl(&repo, tests)
}

/// Get linked tests for a requirement using the default Diesel repository.
pub fn get_linked_tests_for_requirement(
    req_id: i32,
) -> Result<Vec<DecoratedTest>, RepoError> {
    let repo = DieselRepo::new();
    get_linked_tests_for_requirement_impl(&repo, req_id)
}

/// Decorate a list of requirements using the provided repository for lookups.
fn decorate_requirements_impl<R: Repository>(
    repo: &R,
    reqs: Vec<Requirement>,
) -> Vec<DecoratedRequirement> {
    reqs
        .into_iter()
        .map(|r| {
            let verification = repo
                .get_verification_by_id(r.req_verification)
                .map(|v| v.verification_name)
                .unwrap_or_else(|_| format!("Unknown Verification ({})", r.req_verification));

            let status = repo
                .get_status_by_id(r.req_current_status)
                .map(|s| s.st_title)
                .unwrap_or_else(|_| format!("Unknown Status ({})", r.req_current_status));

            let author = if r.req_author != 0 {
                repo
                    .get_user_by_id(r.req_author)
                    .map(|u| u.user_name)
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let reviewer = if r.req_reviewer != 0 {
                repo
                    .get_user_by_id(r.req_reviewer)
                    .map(|u| u.user_name)
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let category = repo
                .get_category_by_id(r.req_category)
                .map(|c| c.cat_title)
                .unwrap_or_else(|_| format!("Unknown Category ({})", r.req_category));

            let applicability = repo
                .get_applicability_by_id(r.req_applicability)
                .map(|a| a.app_title)
                .unwrap_or_else(|_| {
                    format!("Unknown Applicability ({})", r.req_applicability)
                });

            let parent_title = if r.req_parent != 0 {
                match repo.get_requirement_by_id(r.req_parent) {
                    Ok(parent_req) => parent_req.req_title,
                    Err(_) => "[Deleted Parent]".to_string(),
                }
            } else {
                String::new()
            };

            DecoratedRequirement {
                req_id: r.req_id,
                req_title: r.req_title,
                req_verification: verification,
                req_description: r.req_description,
                req_current_status: status,
                req_current_status_id: r.req_current_status,
                req_author: author,
                req_reviewer: reviewer,
                req_link: r.req_link,
                req_reference: r.req_reference,
                req_category: category,
                req_applicability: applicability,
                req_parent_id: r.req_parent,
                req_parent_title: parent_title,
                req_creation_date: r
                    .req_creation_date
                    .format("%d-%m-%Y %H:%M:%S")
                    .to_string(),
                req_update_date: r
                    .req_update_date
                    .format("%d-%m-%Y %H:%M:%S")
                    .to_string(),
                req_deadline_date: r
                    .req_deadline_date
                    .format("%d-%m-%Y %H:%M:%S")
                    .to_string(),
                req_justification: r.req_justification,
                project_id: r.project_id,
            }
        })
        .collect()
}

/// Decorate a list of tests using repository lookups.
 fn decorate_tests_impl<R: Repository>(
    repo: &R,
    tests: Vec<Test>,
) -> Vec<DecoratedTest> {
    tests
        .into_iter()
        .map(|t| {
            let status = repo
                .get_status_by_id(t.test_status)
                .map(|s| s.st_title)
                .unwrap_or_else(|_| format!("Unknown Status ({})", t.test_status));

            let parent_title = if t.test_parent != 0 {
                repo
                    .get_test_by_id(t.test_parent)
                    .map(|p| p.test_name)
                    .unwrap_or_default()
            } else {
                String::new()
            };

            DecoratedTest {
                test_id: t.test_id,
                test_name: t.test_name,
                test_description: t.test_description,
                test_source: t.test_source,
                test_status: status,
                test_status_id: t.test_status,
                test_parent_id: t.test_parent,
                test_parent_title: parent_title,
                project_id: t.project_id,
            }
        })
        .collect()
}

/// Retrieve tests linked to a requirement and return them decorated.
fn get_linked_tests_for_requirement_impl<R: Repository>(
    repo: &R,
    req_id: i32,
) -> Result<Vec<DecoratedTest>, RepoError> {
    let requirement = repo.get_requirement_by_id(req_id)?;
    let matrix = repo.get_matrix_by_project(requirement.project_id)?;

    let test_ids: Vec<i32> = matrix
        .into_iter()
        .filter(|m| m.matrix_req_id == req_id)
        .map(|m| m.matrix_test_id)
        .collect();

    if test_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut tests = Vec::new();
    for id in test_ids {
        tests.push(repo.get_test_by_id(id)?);
    }

    Ok(decorate_tests_impl(repo, tests))
}
