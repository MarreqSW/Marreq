use crate::models::*;
use crate::repository::{errors::RepoError, DieselRepo, Repository};

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
pub fn get_linked_tests_for_requirement(req_id: i32) -> Result<Vec<DecoratedTest>, RepoError> {
    let repo = DieselRepo::new();
    get_linked_tests_for_requirement_impl(&repo, req_id)
}

/// Decorate a list of requirements using the provided repository for lookups.
fn decorate_requirements_impl<R: Repository>(
    repo: &R,
    reqs: Vec<Requirement>,
) -> Vec<DecoratedRequirement> {
    reqs.into_iter()
        .map(|r| {
            let verification = repo
                .get_verification_by_id(r.req_verification)
                .map(|v| v.verification_name)
                .unwrap_or_else(|_| format!("Unknown Verification ({})", r.req_verification));

            let status = repo
                .get_requirement_status_by_id(r.req_current_status)
                .map(|s| s.req_st_title)
                .unwrap_or_else(|_| format!("Unknown Status ({})", r.req_current_status));

            let author = if r.req_author != 0 {
                repo.get_user_by_id(r.req_author)
                    .map(|u| u.user_name)
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let reviewer = if r.req_reviewer != 0 {
                repo.get_user_by_id(r.req_reviewer)
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
                .unwrap_or_else(|_| format!("Unknown Applicability ({})", r.req_applicability));

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
                req_verification_id: r.req_verification,
                req_description: r.req_description,
                req_current_status: status,
                req_current_status_id: r.req_current_status,
                req_author: author,
                req_author_id: r.req_author,
                req_reviewer: reviewer,
                req_reviewer_id: r.req_reviewer,
                req_link: r.req_link,
                req_reference: r.req_reference,
                req_category: category,
                req_category_id: r.req_category,
                req_applicability: applicability,
                req_applicability_id: r.req_applicability,
                req_parent_id: r.req_parent,
                req_parent_title: parent_title,
                req_creation_date: r.req_creation_date.format("%d-%m-%Y %H:%M:%S").to_string(),
                req_update_date: r.req_update_date.format("%d-%m-%Y %H:%M:%S").to_string(),
                req_deadline_date: r.req_deadline_date.format("%d-%m-%Y %H:%M:%S").to_string(),
                req_justification: r.req_justification,
                project_id: r.project_id,
            }
        })
        .collect()
}

/// Decorate a list of tests using repository lookups.
fn decorate_tests_impl<R: Repository>(repo: &R, tests: Vec<Test>) -> Vec<DecoratedTest> {
    tests
        .into_iter()
        .map(|t| {
            let status = repo
                .get_test_status_by_id(t.test_status)
                .map(|s| s.test_st_title)
                .unwrap_or_else(|_| format!("Unknown Status ({})", t.test_status));

            let parent_title = if t.test_parent != 0 {
                repo.get_test_by_id(t.test_parent)
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
                test_reference: t.test_reference,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use chrono::{NaiveDate, NaiveDateTime};

    fn dt() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    #[test]
    fn decorate_requirements_impl_covers_branches() {
        let now = dt();
        let mut repo = DieselRepoMock::default();

        // Lookup data
        repo.requirement_statuses.insert(
            1,
            RequirementStatus {
                req_st_id: 1,
                req_st_title: "Open".into(),
                req_st_description: String::new(),
                req_st_short_name: String::new(),
            },
        );
        repo.verifications.insert(
            1,
            Verification {
                verification_id: 1,
                verification_name: "Analysis".into(),
                verification_description: String::new(),
                project_id: 1,
            },
        );
        repo.categories.insert(
            1,
            Category {
                cat_id: 1,
                cat_title: "Cat".into(),
                cat_description: String::new(),
                cat_tag: String::new(),
                project_id: 1,
            },
        );
        repo.applicability.insert(
            1,
            Applicability {
                app_id: 1,
                app_title: "App".into(),
                app_description: String::new(),
                app_tag: String::new(),
                project_id: 1,
            },
        );

        repo.users.insert(
            1,
            User {
                user_id: 1,
                user_username: "a".into(),
                user_name: "Author".into(),
                user_email: String::new(),
                user_creation_date: now,
                user_last_login: now,
                user_password: String::new(),
                is_admin: false,
            },
        );
        repo.users.insert(
            2,
            User {
                user_id: 2,
                user_username: "b".into(),
                user_name: "Reviewer".into(),
                user_email: String::new(),
                user_creation_date: now,
                user_last_login: now,
                user_password: String::new(),
                is_admin: false,
            },
        );

        // Parent requirement for branch coverage
        repo.requirements.insert(
            31,
            Requirement {
                req_id: 31,
                req_title: "Parent".into(),
                req_description: String::new(),
                req_verification: 1,
                req_current_status: 1,
                req_author: 1,
                req_reviewer: 2,
                req_link: String::new(),
                req_reference: String::new(),
                req_category: 1,
                req_parent: 0,
                req_creation_date: now,
                req_update_date: now,
                req_deadline_date: now,
                req_applicability: 1,
                req_justification: None,
                project_id: 1,
            },
        );

        let r1 = Requirement {
            req_id: 1,
            req_title: "R1".into(),
            req_description: String::new(),
            req_verification: 1,
            req_current_status: 1,
            req_author: 1,
            req_reviewer: 2,
            req_link: String::new(),
            req_reference: String::new(),
            req_category: 1,
            req_parent: 0,
            req_creation_date: now,
            req_update_date: now,
            req_deadline_date: now,
            req_applicability: 1,
            req_justification: None,
            project_id: 1,
        };

        let r2 = Requirement {
            req_id: 2,
            req_title: "R2".into(),
            req_description: String::new(),
            req_verification: 1,
            req_current_status: 1,
            req_author: 0,
            req_reviewer: 0,
            req_link: String::new(),
            req_reference: String::new(),
            req_category: 1,
            req_parent: 31,
            req_creation_date: now,
            req_update_date: now,
            req_deadline_date: now,
            req_applicability: 1,
            req_justification: None,
            project_id: 1,
        };

        let r3 = Requirement {
            req_id: 3,
            req_title: "R3".into(),
            req_description: String::new(),
            req_verification: 99,
            req_current_status: 99,
            req_author: 99,
            req_reviewer: 98,
            req_link: String::new(),
            req_reference: String::new(),
            req_category: 99,
            req_parent: 32,
            req_creation_date: now,
            req_update_date: now,
            req_deadline_date: now,
            req_applicability: 99,
            req_justification: None,
            project_id: 1,
        };

        let decorated = decorate_requirements_impl(&repo, vec![r1, r2, r3]);

        assert_eq!(decorated.len(), 3);
        let d1 = &decorated[0];
        assert_eq!(d1.req_verification, "Analysis");
        assert_eq!(d1.req_current_status, "Open");
        assert_eq!(d1.req_author, "Author");
        assert_eq!(d1.req_reviewer, "Reviewer");
        assert_eq!(d1.req_category, "Cat");
        assert_eq!(d1.req_applicability, "App");
        assert_eq!(d1.req_parent_title, "");

        let d2 = &decorated[1];
        assert_eq!(d2.req_author, "");
        assert_eq!(d2.req_reviewer, "");
        assert_eq!(d2.req_parent_title, "Parent");

        let d3 = &decorated[2];
        assert!(d3.req_verification.starts_with("Unknown Verification"));
        assert!(d3.req_current_status.starts_with("Unknown Status"));
        assert_eq!(d3.req_author, "");
        assert_eq!(d3.req_reviewer, "");
        assert!(d3.req_category.starts_with("Unknown Category"));
        assert!(d3.req_applicability.starts_with("Unknown Applicability"));
        assert_eq!(d3.req_parent_title, "[Deleted Parent]");
    }

    #[test]
    fn decorate_tests_impl_covers_branches() {
        let mut repo = DieselRepoMock::default();
        repo.test_statuses.insert(
            1,
            TestStatus {
                test_st_id: 1,
                test_st_title: "Open".into(),
                test_st_description: String::new(),
                test_st_short_name: String::new(),
            },
        );
        // parent test for branch
        repo.tests.insert(
            10,
            Test {
                test_id: 10,
                test_name: "Parent".into(),
                test_description: String::new(),
                test_source: String::new(),
                test_reference: "TEST-10".into(),
                test_status: 1,
                test_parent: 0,
                project_id: 1,
            },
        );

        let t1 = Test {
            test_id: 20,
            test_name: "T1".into(),
            test_description: String::new(),
            test_source: String::new(),
            test_reference: "TEST-20".into(),
            test_status: 1,
            test_parent: 0,
            project_id: 1,
        };
        let t2 = Test {
            test_id: 21,
            test_name: "T2".into(),
            test_description: String::new(),
            test_source: String::new(),
            test_reference: "TEST-21".into(),
            test_status: 99,
            test_parent: 10,
            project_id: 1,
        };
        let t3 = Test {
            test_id: 22,
            test_name: "T3".into(),
            test_description: String::new(),
            test_source: String::new(),
            test_reference: "TEST-22".into(),
            test_status: 1,
            test_parent: 999,
            project_id: 1,
        };

        let decorated = decorate_tests_impl(&repo, vec![t1, t2, t3]);
        assert_eq!(decorated.len(), 3);
        assert_eq!(decorated[0].test_status, "Open");
        assert_eq!(decorated[0].test_parent_title, "");
        assert_eq!(decorated[1].test_status, "Unknown Status (99)");
        assert_eq!(decorated[1].test_parent_title, "Parent");
        assert_eq!(decorated[2].test_parent_title, "");
    }

    #[test]
    fn get_linked_tests_for_requirement_impl_works() {
        let now = dt();
        let mut repo = DieselRepoMock::default();
        repo.requirement_statuses.insert(
            1,
            RequirementStatus {
                req_st_id: 1,
                req_st_title: "Open".into(),
                req_st_description: String::new(),
                req_st_short_name: String::new(),
            },
        );
        repo.test_statuses.insert(
            1,
            TestStatus {
                test_st_id: 1,
                test_st_title: "Open".into(),
                test_st_description: String::new(),
                test_st_short_name: String::new(),
            },
        );
        let req = Requirement {
            req_id: 1,
            req_title: "R".into(),
            req_description: String::new(),
            req_verification: 0,
            req_current_status: 0,
            req_author: 0,
            req_reviewer: 0,
            req_link: String::new(),
            req_reference: String::new(),
            req_category: 0,
            req_parent: 0,
            req_creation_date: now,
            req_update_date: now,
            req_deadline_date: now,
            req_applicability: 0,
            req_justification: None,
            project_id: 1,
        };
        let test = Test {
            test_id: 10,
            test_name: "T".into(),
            test_description: String::new(),
            test_source: String::new(),
            test_reference: "TEST-10".into(),
            test_status: 1,
            test_parent: 0,
            project_id: 1,
        };
        repo.requirements.insert(1, req);
        repo.tests.insert(10, test);
        repo.matrices.push(Matrix {
            matrix_req_id: 1,
            matrix_test_id: 10,
            matrix_creation_date: now,
            project_id: 1,
        });

        let result = get_linked_tests_for_requirement_impl(&repo, 1).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].test_name, "T");
        assert_eq!(result[0].test_status, "Open");
    }

    #[test]
    fn get_linked_tests_for_requirement_impl_empty_when_no_links() {
        let now = dt();
        let mut repo = DieselRepoMock::default();
        let req = Requirement {
            req_id: 2,
            req_title: "R".into(),
            req_description: String::new(),
            req_verification: 0,
            req_current_status: 0,
            req_author: 0,
            req_reviewer: 0,
            req_link: String::new(),
            req_reference: String::new(),
            req_category: 0,
            req_parent: 0,
            req_creation_date: now,
            req_update_date: now,
            req_deadline_date: now,
            req_applicability: 0,
            req_justification: None,
            project_id: 1,
        };
        repo.requirements.insert(2, req);
        // matrix for different requirement
        repo.matrices.push(Matrix {
            matrix_req_id: 99,
            matrix_test_id: 50,
            matrix_creation_date: now,
            project_id: 1,
        });

        let result = get_linked_tests_for_requirement_impl(&repo, 2).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn get_linked_tests_for_requirement_impl_errors_when_req_missing() {
        let repo = DieselRepoMock::default();
        let err = get_linked_tests_for_requirement_impl(&repo, 123).unwrap_err();
        matches!(err, RepoError::NotFound);
    }

    #[test]
    fn get_linked_tests_for_requirement_impl_errors_when_test_missing() {
        let now = dt();
        let mut repo = DieselRepoMock::default();
        let req = Requirement {
            req_id: 3,
            req_title: "R".into(),
            req_description: String::new(),
            req_verification: 0,
            req_current_status: 0,
            req_author: 0,
            req_reviewer: 0,
            req_link: String::new(),
            req_reference: String::new(),
            req_category: 0,
            req_parent: 0,
            req_creation_date: now,
            req_update_date: now,
            req_deadline_date: now,
            req_applicability: 0,
            req_justification: None,
            project_id: 1,
        };
        repo.requirements.insert(3, req);
        repo.matrices.push(Matrix {
            matrix_req_id: 3,
            matrix_test_id: 999,
            matrix_creation_date: now,
            project_id: 1,
        });

        assert!(get_linked_tests_for_requirement_impl(&repo, 3).is_err());
    }
}
