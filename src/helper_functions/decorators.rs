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
pub fn decorate_tests_with_repo<R: Repository>(
    repo: &R,
    tests: Vec<TestCase>,
) -> Vec<DecoratedTestCase> {
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
            let verification_ids = repo
                .get_verification_method_ids_for_requirement(r.id)
                .unwrap_or_default();
            let verification = if verification_ids.is_empty() {
                "—".to_string()
            } else {
                verification_ids
                    .iter()
                    .map(|id| {
                        repo.get_verification_by_id(*id)
                            .map(|v| v.title)
                            .unwrap_or_else(|_| format!("Unknown Verification ({})", id))
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            let status = repo
                .get_requirement_status_by_id(r.status_id)
                .map(|s| s.title)
                .unwrap_or_else(|_| format!("Unknown Status ({})", r.status_id));

            let author = if r.author_id != 0 {
                repo.get_user_by_id(r.author_id)
                    .map(|u| u.name)
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let reviewer = if r.reviewer_id != 0 {
                repo.get_user_by_id(r.reviewer_id)
                    .map(|u| u.name)
                    .unwrap_or_default()
            } else {
                String::new()
            };

            let category = repo
                .get_category_by_id(r.category_id)
                .map(|c| c.title)
                .unwrap_or_else(|_| format!("Unknown Category ({})", r.category_id));

            let applicability = repo
                .get_applicability_by_id(r.applicability_id)
                .map(|a| a.title)
                .unwrap_or_else(|_| format!("Unknown Applicability ({})", r.applicability_id));

            let (parent_title, parent_ref, parent_desc, parent_status, parent_category) =
                if let Some(parent_id) = r.parent_id {
                    match repo.get_requirement_by_id(parent_id) {
                        Ok(parent_req) => {
                            let p_status = repo
                                .get_requirement_status_by_id(parent_req.status_id)
                                .map(|s| s.title)
                                .unwrap_or_else(|_| {
                                    format!("Unknown Status ({})", parent_req.status_id)
                                });
                            let p_category = repo
                                .get_category_by_id(parent_req.category_id)
                                .map(|c| c.title)
                                .unwrap_or_else(|_| {
                                    format!("Unknown Category ({})", parent_req.category_id)
                                });
                            (
                                parent_req.title,
                                parent_req.reference_code,
                                parent_req.description,
                                p_status,
                                p_category,
                            )
                        }
                        Err(_) => (
                            "[Deleted Parent]".to_string(),
                            String::new(),
                            String::new(),
                            String::new(),
                            String::new(),
                        ),
                    }
                } else {
                    (
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                    )
                };

            DecoratedRequirement {
                id: r.id,
                title: r.title,
                verification_method_id: verification,
                req_verification_ids: verification_ids,
                description: r.description,
                status_id: status,
                req_current_status_id: r.status_id,
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
                req_parent_reference_code: parent_ref,
                req_parent_description: parent_desc,
                req_parent_status_id: parent_status,
                req_parent_category_id: parent_category,
                creation_date: r.creation_date.format("%d-%m-%Y %H:%M:%S").to_string(),
                update_date: r.update_date.format("%d-%m-%Y %H:%M:%S").to_string(),
                deadline_date: r
                    .deadline_date
                    .map(|d| d.format("%d-%m-%Y %H:%M:%S").to_string())
                    .unwrap_or_default(),
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
                .get_test_status_by_id(t.status_id)
                .map(|s| s.title)
                .unwrap_or_else(|_| format!("Unknown Status ({})", t.status_id));

            let parent_title = if let Some(parent_id) = t.parent_id {
                repo.get_test_by_id(parent_id)
                    .map(|p| p.name)
                    .unwrap_or_default()
            } else {
                String::new()
            };

            DecoratedTestCase {
                id: t.id,
                name: t.name,
                description: t.description,
                source: t.source,
                reference_code: t.reference_code,
                status_id: status,
                test_status_id: t.status_id,
                test_parent_id: t.parent_id,
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
        .filter(|m| m.req_id == id)
        .map(|m| m.test_id)
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
                id: 1,
                title: "Open".into(),
                description: String::new(),
                tag: String::new(),
                project_id: 1,
            },
        );
        repo.verifications.insert(
            1,
            VerificationMethod {
                id: 1,
                title: "Analysis".into(),
                description: String::new(),
                tag: "ANALYSIS".into(),
                project_id: 1,
            },
        );
        repo.requirement_verification_methods.push((1, 1));
        repo.requirement_verification_methods.push((2, 1));
        repo.requirement_verification_methods.push((3, 99));
        repo.categories.insert(
            1,
            Category {
                id: 1,
                title: "Cat".into(),
                description: String::new(),
                tag: String::new(),
                project_id: 1,
            },
        );
        repo.applicability.insert(
            1,
            Applicability {
                id: 1,
                title: "App".into(),
                description: String::new(),
                tag: String::new(),
                project_id: 1,
            },
        );

        repo.users.insert(
            1,
            User {
                id: 1,
                username: "a".into(),
                name: "Author".into(),
                email: String::new(),
                creation_date: now,
                last_login: now,
                password_hash: String::new(),
                is_admin: false,
            },
        );
        repo.users.insert(
            2,
            User {
                id: 2,
                username: "b".into(),
                name: "Reviewer".into(),
                email: String::new(),
                creation_date: now,
                last_login: now,
                password_hash: String::new(),
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
                status_id: 1,
                author_id: 1,
                reviewer_id: 2,
                reference_code: String::new(),
                category_id: 1,
                parent_id: None,
                creation_date: now,
                update_date: now,
                deadline_date: Some(now),
                applicability_id: 1,
                justification: None,
                project_id: 1,
            },
        );

        let r1 = Requirement {
            id: 1,
            title: "R1".into(),
            description: String::new(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 2,
            reference_code: String::new(),
            category_id: 1,
            parent_id: None,
            creation_date: now,
            update_date: now,
            deadline_date: Some(now),
            applicability_id: 1,
            justification: None,
            project_id: 1,
        };

        let r2 = Requirement {
            id: 2,
            title: "R2".into(),
            description: String::new(),
            status_id: 1,
            author_id: 0,
            reviewer_id: 0,
            reference_code: String::new(),
            category_id: 1,
            parent_id: Some(31),
            creation_date: now,
            update_date: now,
            deadline_date: Some(now),
            applicability_id: 1,
            justification: None,
            project_id: 1,
        };

        let r3 = Requirement {
            id: 3,
            title: "R3".into(),
            description: String::new(),
            status_id: 99,
            author_id: 99,
            reviewer_id: 98,
            reference_code: String::new(),
            category_id: 99,
            parent_id: Some(32),
            creation_date: now,
            update_date: now,
            deadline_date: Some(now),
            applicability_id: 99,
            justification: None,
            project_id: 1,
        };

        let decorated = decorate_requirements_impl(&repo, vec![r1, r2, r3]);

        assert_eq!(decorated.len(), 3);
        let d1 = &decorated[0];
        assert_eq!(d1.verification_method_id, "Analysis");
        assert_eq!(d1.status_id, "Open");
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
        assert!(d3
            .verification_method_id
            .starts_with("Unknown Verification"));
        assert!(d3.status_id.starts_with("Unknown Status"));
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
                id: 1,
                title: "Open".into(),
                description: String::new(),
                tag: String::new(),
                project_id: 1,
            },
        );
        // parent test for branch
        repo.tests.insert(
            10,
            TestCase {
                id: 10,
                name: "Parent".into(),
                description: String::new(),
                source: String::new(),
                reference_code: "TEST-10".into(),
                status_id: 1,
                parent_id: None,
                project_id: 1,
            },
        );

        let t1 = TestCase {
            id: 20,
            name: "T1".into(),
            description: String::new(),
            source: String::new(),
            reference_code: "TEST-20".into(),
            status_id: 1,
            parent_id: None,
            project_id: 1,
        };
        let t2 = TestCase {
            id: 21,
            name: "T2".into(),
            description: String::new(),
            source: String::new(),
            reference_code: "TEST-21".into(),
            status_id: 99,
            parent_id: Some(10),
            project_id: 1,
        };
        let t3 = TestCase {
            id: 22,
            name: "T3".into(),
            description: String::new(),
            source: String::new(),
            reference_code: "TEST-22".into(),
            status_id: 1,
            parent_id: Some(999),
            project_id: 1,
        };

        let decorated = decorate_tests_impl(&repo, vec![t1, t2, t3]);
        assert_eq!(decorated.len(), 3);
        assert_eq!(decorated[0].status_id, "Open");
        assert_eq!(decorated[0].test_parent_title, "");
        assert_eq!(decorated[1].status_id, "Unknown Status (99)");
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
                id: 1,
                title: "Open".into(),
                description: String::new(),
                tag: String::new(),
                project_id: 1,
            },
        );
        repo.test_statuses.insert(
            1,
            TestStatus {
                id: 1,
                title: "Open".into(),
                description: String::new(),
                tag: String::new(),
                project_id: 1,
            },
        );
        let req = Requirement {
            id: 1,
            title: "R".into(),
            description: String::new(),
            status_id: 0,
            author_id: 0,
            reviewer_id: 0,
            reference_code: String::new(),
            category_id: 0,
            parent_id: None,
            creation_date: now,
            update_date: now,
            deadline_date: Some(now),
            applicability_id: 0,
            justification: None,
            project_id: 1,
        };
        let test = TestCase {
            id: 10,
            name: "T".into(),
            description: String::new(),
            source: String::new(),
            reference_code: "TEST-10".into(),
            status_id: 1,
            parent_id: None,
            project_id: 1,
        };
        repo.requirements.insert(1, req);
        repo.tests.insert(10, test);
        repo.matrices.push(MatrixLink {
            req_id: 1,
            test_id: 10,
            creation_date: now,
            project_id: 1,
        });

        let result = get_linked_tests_for_requirement_impl(&repo, 1).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "T");
        assert_eq!(result[0].status_id, "Open");
    }

    #[test]
    fn get_linked_tests_for_requirement_impl_empty_when_no_links() {
        let now = dt();
        let mut repo = DieselRepoMock::default();
        let req = Requirement {
            id: 2,
            title: "R".into(),
            description: String::new(),
            status_id: 0,
            author_id: 0,
            reviewer_id: 0,
            reference_code: String::new(),
            category_id: 0,
            parent_id: None,
            creation_date: now,
            update_date: now,
            deadline_date: Some(now),
            applicability_id: 0,
            justification: None,
            project_id: 1,
        };
        repo.requirements.insert(2, req);
        // matrix for different requirement
        repo.matrices.push(MatrixLink {
            req_id: 99,
            test_id: 50,
            creation_date: now,
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
            status_id: 0,
            author_id: 0,
            reviewer_id: 0,
            reference_code: String::new(),
            category_id: 0,
            parent_id: None,
            creation_date: now,
            update_date: now,
            deadline_date: Some(now),
            applicability_id: 0,
            justification: None,
            project_id: 1,
        };
        repo.requirements.insert(3, req);
        repo.matrices.push(MatrixLink {
            req_id: 3,
            test_id: 999,
            creation_date: now,
            project_id: 1,
        });

        assert!(get_linked_tests_for_requirement_impl(&repo, 3).is_err());
    }
}
