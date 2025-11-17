use crate::models::*;
use crate::repository::{errors::RepoError, DieselRepo, Repository};

/// Decorate requirements using the default Diesel repository.
pub fn decorate_requirements(reqs: Vec<Requirement>) -> Vec<DecoratedRequirement> {
    let repo = DieselRepo::new();
    decorate_requirements_impl(&repo, reqs)
}

/// Decorate requirements using an explicitly provided repository.
pub fn decorate_requirements_with_repo<R: Repository>(
    repo: &R,
    reqs: Vec<Requirement>,
) -> Vec<DecoratedRequirement> {
    decorate_requirements_impl(repo, reqs)
}

/// Decorate tests using the default Diesel repository.
pub fn decorate_tests(tests: Vec<TestCase>) -> Vec<DecoratedTestCase> {
    let repo = DieselRepo::new();
    decorate_tests_impl(&repo, tests)
}

/// Decorate tests using an explicitly provided repository.
pub fn decorate_tests_with_repo<R: Repository>(repo: &R, tests: Vec<TestCase>) -> Vec<DecoratedTestCase> {
    decorate_tests_impl(repo, tests)
}

/// Get linked tests for a requirement using the default Diesel repository.
pub fn get_linked_tests_for_requirement(id: i32) -> Result<Vec<DecoratedTestCase>, RepoError> {
    let repo = DieselRepo::new();
    get_linked_tests_for_requirement_impl(&repo, id)
}

/// Retrieve linked tests using an explicitly provided repository.
pub fn get_linked_tests_for_requirement_with_repo<R: Repository>(
    repo: &R,
    id: i32,
) -> Result<Vec<DecoratedTestCase>, RepoError> {
    get_linked_tests_for_requirement_impl(repo, id)
}

/// Decorate a list of requirements using the provided repository for lookups.
fn decorate_requirements_impl<R: Repository>(
    repo: &R,
    reqs: Vec<Requirement>,
) -> Vec<DecoratedRequirement> {
    reqs.into_iter()
        .map(|r| {
            let verification = repo
                .get_verification_by_id(r.verification_method_id)
                .map(|v| v.verification_name)
                .unwrap_or_else(|_| format!("Unknown Verification ({})", r.verification_method_id));

            let status = repo
                .get_requirement_status_by_id(r.current_status_id)
                .map(|s| s.req_st_title)
                .unwrap_or_else(|_| format!("Unknown Status ({})", r.current_status_id));

            let author = if r.author_id != 0 {
                repo.get_user_by_id(r.author_id)
                    .map(|u| u.user_name)
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let reviewer = if r.reviewer_id != 0 {
                repo.get_user_by_id(r.reviewer_id)
                    .map(|u| u.user_name)
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let category = repo
                .get_category_by_id(r.category_id)
                .map(|c| c.cat_title)
                .unwrap_or_else(|_| format!("Unknown Category ({})", r.category_id));

            let applicability = repo
                .get_applicability_by_id(r.applicability_id)
                .map(|a| a.app_title)
                .unwrap_or_else(|_| format!("Unknown Applicability ({})", r.applicability_id));

            let parent_title = if r.parent_id != 0 {
                match repo.get_requirement_by_id(r.parent_id) {
                    Ok(parent_req) => parent_req.title,
                    Err(_) => "[Deleted Parent]".to_string(),
                }
            } else {
                String::new()
            };

            DecoratedRequirement {
                id: r.id,
                title: r.title,
                verification_method_id: verification,
                req_verification_id: r.verification_method_id,
                description: r.description,
                current_status_id: status,
                req_current_status_id: r.current_status_id,
                author_id: author,
                req_author_id: r.author_id,
                reviewer_id: reviewer,
                req_reviewer_id: r.reviewer_id,
                reference_code: r.reference_code,
                category_id: category,
                req_category_id: r.category_id,
                applicability_id: applicability,
                req_applicability_id: r.applicability_id,
                req_parent_id: r.parent_id,
                req_parent_title: parent_title,
                creation_date: r.creation_date.format("%d-%m-%Y %H:%M:%S").to_string(),
                update_date: r.update_date.format("%d-%m-%Y %H:%M:%S").to_string(),
                deadline_date: r.deadline_date.format("%d-%m-%Y %H:%M:%S").to_string(),
                justification: r.justification,
                project_id: r.project_id,
            }
        })
        .collect()
}

/// Decorate a list of tests using repository lookups.
fn decorate_tests_impl<R: Repository>(repo: &R, tests: Vec<TestCase>) -> Vec<DecoratedTestCase> {
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

            DecoratedTestCase {
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
    id: i32,
) -> Result<Vec<DecoratedTestCase>, RepoError> {
    let requirement = repo.get_requirement_by_id(id)?;
    let matrix = repo.get_matrix_by_project(requirement.project_id)?;

    let test_ids: Vec<i32> = matrix
        .into_iter()
        .filter(|m| m.matrix_req_id == id)
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
            VerificationMethod {
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
                id: 31,
                title: "Parent".into(),
                description: String::new(),
                verification_method_id: 1,
                current_status_id: 1,
                author_id: 1,
                reviewer_id: 2,
                reference_code: String::new(),
                category_id: 1,
                parent_id: 0,
                creation_date: now,
                update_date: now,
                deadline_date: now,
                applicability_id: 1,
                justification: None,
                project_id: 1,
            },
        );

        let r1 = Requirement {
            id: 1,
            title: "R1".into(),
            description: String::new(),
            verification_method_id: 1,
            current_status_id: 1,
            author_id: 1,
            reviewer_id: 2,
            reference_code: String::new(),
            category_id: 1,
            parent_id: 0,
            creation_date: now,
            update_date: now,
            deadline_date: now,
            applicability_id: 1,
            justification: None,
            project_id: 1,
        };

        let r2 = Requirement {
            id: 2,
            title: "R2".into(),
            description: String::new(),
            verification_method_id: 1,
            current_status_id: 1,
            author_id: 0,
            reviewer_id: 0,
            reference_code: String::new(),
            category_id: 1,
            parent_id: 31,
            creation_date: now,
            update_date: now,
            deadline_date: now,
            applicability_id: 1,
            justification: None,
            project_id: 1,
        };

        let r3 = Requirement {
            id: 3,
            title: "R3".into(),
            description: String::new(),
            verification_method_id: 99,
            current_status_id: 99,
            author_id: 99,
            reviewer_id: 98,
            reference_code: String::new(),
            category_id: 99,
            parent_id: 32,
            creation_date: now,
            update_date: now,
            deadline_date: now,
            applicability_id: 99,
            justification: None,
            project_id: 1,
        };

        let decorated = decorate_requirements_impl(&repo, vec![r1, r2, r3]);

        assert_eq!(decorated.len(), 3);
        let d1 = &decorated[0];
        assert_eq!(d1.verification_method_id, "Analysis");
        assert_eq!(d1.current_status_id, "Open");
        assert_eq!(d1.author_id, "Author");
        assert_eq!(d1.reviewer_id, "Reviewer");
        assert_eq!(d1.category_id, "Cat");
        assert_eq!(d1.applicability_id, "App");
        assert_eq!(d1.req_parent_title, "");

        let d2 = &decorated[1];
        assert_eq!(d2.author_id, "");
        assert_eq!(d2.reviewer_id, "");
        assert_eq!(d2.req_parent_title, "Parent");

        let d3 = &decorated[2];
        assert!(d3.verification_method_id.starts_with("Unknown Verification"));
        assert!(d3.current_status_id.starts_with("Unknown Status"));
        assert_eq!(d3.author_id, "");
        assert_eq!(d3.reviewer_id, "");
        assert!(d3.category_id.starts_with("Unknown Category"));
        assert!(d3.applicability_id.starts_with("Unknown Applicability"));
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
            TestCase {
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

        let t1 = TestCase {
            test_id: 20,
            test_name: "T1".into(),
            test_description: String::new(),
            test_source: String::new(),
            test_reference: "TEST-20".into(),
            test_status: 1,
            test_parent: 0,
            project_id: 1,
        };
        let t2 = TestCase {
            test_id: 21,
            test_name: "T2".into(),
            test_description: String::new(),
            test_source: String::new(),
            test_reference: "TEST-21".into(),
            test_status: 99,
            test_parent: 10,
            project_id: 1,
        };
        let t3 = TestCase {
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
            id: 1,
            title: "R".into(),
            description: String::new(),
            verification_method_id: 0,
            current_status_id: 0,
            author_id: 0,
            reviewer_id: 0,
            reference_code: String::new(),
            category_id: 0,
            parent_id: 0,
            creation_date: now,
            update_date: now,
            deadline_date: now,
            applicability_id: 0,
            justification: None,
            project_id: 1,
        };
        let test = TestCase {
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
            id: 2,
            title: "R".into(),
            description: String::new(),
            verification_method_id: 0,
            current_status_id: 0,
            author_id: 0,
            reviewer_id: 0,
            reference_code: String::new(),
            category_id: 0,
            parent_id: 0,
            creation_date: now,
            update_date: now,
            deadline_date: now,
            applicability_id: 0,
            justification: None,
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
            id: 3,
            title: "R".into(),
            description: String::new(),
            verification_method_id: 0,
            current_status_id: 0,
            author_id: 0,
            reviewer_id: 0,
            reference_code: String::new(),
            category_id: 0,
            parent_id: 0,
            creation_date: now,
            update_date: now,
            deadline_date: now,
            applicability_id: 0,
            justification: None,
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
