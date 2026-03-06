// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::models::*;
use crate::repository::{errors::RepoError, DieselRepo, Repository};

/// Decorate requirements using the default Diesel repository.
pub fn decorate_requirements(reqs: Vec<Requirement>) -> Vec<DecoratedRequirement> {
    let repo = DieselRepo::new().expect("Database connection not available");
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
pub fn decorate_verifications(verifications: Vec<Verification>) -> Vec<DecoratedVerification> {
    let repo = DieselRepo::new().expect("Database connection not available");
    decorate_verifications_impl(&repo, verifications)
}

/// Decorate verifications using an explicitly provided repository.
pub fn decorate_verifications_with_repo<R: Repository>(
    repo: &R,
    verifications: Vec<Verification>,
) -> Vec<DecoratedVerification> {
    decorate_verifications_impl(repo, verifications)
}

/// Get linked verifications for a requirement using the default Diesel repository.
pub fn get_linked_verifications_for_requirement(
    id: i32,
) -> Result<Vec<DecoratedVerification>, RepoError> {
    let repo = DieselRepo::new().map_err(|e| RepoError::Pool(e.to_string()))?;
    get_linked_verifications_for_requirement_impl(&repo, id)
}

/// Retrieve linked verifications using an explicitly provided repository.
pub fn get_linked_verifications_for_requirement_with_repo<R: Repository>(
    repo: &R,
    id: i32,
) -> Result<Vec<DecoratedVerification>, RepoError> {
    get_linked_verifications_for_requirement_impl(repo, id)
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
                        repo.get_verification_method_by_id(*id)
                            .map(|v| v.title)
                            .unwrap_or_else(|_| format!("Unknown Verification ({})", id))
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            let (status, status_tag_color) = repo
                .get_requirement_status_by_id(r.status_id)
                .map(|s| (s.title, s.tag_color))
                .unwrap_or_else(|_| (format!("Unknown Status ({})", r.status_id), None));

            let author = repo
                .get_user_by_id(r.author_id)
                .map(|u| u.name)
                .unwrap_or_default();

            let reviewer = repo
                .get_user_by_id(r.reviewer_id)
                .map(|u| u.name)
                .unwrap_or_default();

            let category = repo
                .get_category_by_id(r.category_id)
                .map(|c| c.title)
                .unwrap_or_else(|_| format!("Unknown Category ({})", r.category_id));

            let applicability = repo
                .get_applicability_by_id(r.applicability_id)
                .map(|a| a.title)
                .unwrap_or_else(|_| format!("Unknown Applicability ({})", r.applicability_id));

            // All parents from requirement_version_links (source = this version).
            let req_parents: Vec<ReqParentDisplay> = r
                .current_version_id
                .and_then(|vid| repo.list_links_by_source_version(vid).ok())
                .unwrap_or_default()
                .into_iter()
                .filter_map(|link| {
                    let ver = repo
                        .get_requirement_version_by_id(link.target_version_id)
                        .ok()?;
                    let parent_req = repo.get_requirement_by_id(ver.requirement_id).ok()?;
                    Some(ReqParentDisplay {
                        id: parent_req.id,
                        reference_code: parent_req.reference_code.clone(),
                        title: parent_req.title.clone(),
                    })
                })
                .collect();

            // Single-parent fields: first from req_parents (for tree / backward compat).
            let first_parent_id = req_parents.first().map(|p| p.id);
            let (
                parent_title,
                parent_ref,
                parent_desc,
                parent_status,
                parent_category,
                req_parent_status_tag_color,
            ) = if let Some(parent_id) = first_parent_id {
                match repo.get_requirement_by_id(parent_id) {
                    Ok(parent_req) => {
                        let (p_status, p_tag_color) = repo
                            .get_requirement_status_by_id(parent_req.status_id)
                            .map(|s| (s.title, s.tag_color))
                            .unwrap_or_else(|_| {
                                (format!("Unknown Status ({})", parent_req.status_id), None)
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
                            p_tag_color,
                        )
                    }
                    Err(_) => (
                        "[Deleted Parent]".to_string(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                        None,
                    ),
                }
            } else {
                (
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    None,
                )
            };

            DecoratedRequirement {
                id: r.id,
                current_version_id: r.current_version_id,
                title: r.title,
                verification_method_id: verification,
                req_verification_ids: verification_ids,
                description: r.description,
                status_id: status,
                req_current_status_id: r.status_id,
                status_tag_color,
                author_id: author,
                req_author_id: r.author_id,
                reviewer_id: reviewer,
                req_reviewer_id: r.reviewer_id,
                reference_code: r.reference_code,
                category_id: category,
                req_category_id: r.category_id,
                applicability_id: applicability,
                req_applicability_id: r.applicability_id,
                req_parent_id: first_parent_id,
                req_parent_title: parent_title,
                req_parents: req_parents.clone(),
                req_parent_reference_code: parent_ref,
                req_parent_description: parent_desc,
                req_parent_status_id: parent_status,
                req_parent_status_tag_color,
                req_parent_category_id: parent_category,
                creation_date: r.creation_date.format("%d-%m-%Y %H:%M:%S").to_string(),
                update_date: r.update_date.format("%d-%m-%Y %H:%M:%S").to_string(),
                deadline_date: r
                    .deadline_date
                    .map(|d| d.format("%d-%m-%Y %H:%M:%S").to_string())
                    .unwrap_or_default(),
                justification: r.justification,
                project_id: r.project_id,
                approval_state: r.approval_state.clone(),
                approved_by: r.approved_by,
                approved_at: r.approved_at,
                custom_fields: r.custom_fields.clone(),
            }
        })
        .collect()
}

/// Maps test status title to CSS class variant: passed, failed, proposal, draft, default.
fn test_status_title_to_variant(title: &str) -> &'static str {
    match title.trim().to_lowercase().as_str() {
        "passed" => "passed",
        "failed" => "failed",
        "pending" => "proposal",
        "in progress" => "draft",
        _ => "default",
    }
}

/// Decorate a list of verifications using repository lookups.
fn decorate_verifications_impl<R: Repository>(
    repo: &R,
    verifications: Vec<Verification>,
) -> Vec<DecoratedVerification> {
    verifications
        .into_iter()
        .map(|v| {
            let (status, status_tag_color) = repo
                .get_verification_status_by_id(v.status_id)
                .map(|s| (s.title, s.tag_color))
                .unwrap_or_else(|_| (format!("Unknown Status ({})", v.status_id), None));

            let empty = (
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                None,
            );
            let (
                parent_title,
                parent_ref,
                parent_desc,
                parent_status,
                parent_status_variant,
                parent_source,
                verification_parent_status_tag_color,
            ) = if let Some(parent_id) = v.parent_id {
                repo.get_verification_by_id(parent_id)
                    .map(|p| {
                        let (p_status, p_tag_color) = repo
                            .get_verification_status_by_id(p.status_id)
                            .map(|s| (s.title, s.tag_color))
                            .unwrap_or_else(|_| {
                                (format!("Unknown Status ({})", p.status_id), None)
                            });
                        (
                            p.name,
                            p.reference_code,
                            p.description,
                            p_status.clone(),
                            test_status_title_to_variant(&p_status).to_string(),
                            p.source,
                            p_tag_color,
                        )
                    })
                    .unwrap_or(empty)
            } else {
                empty
            };

            DecoratedVerification {
                id: v.id,
                name: v.name,
                description: v.description,
                source: v.source,
                reference_code: v.reference_code,
                status_id: status.clone(),
                status_variant: test_status_title_to_variant(&status).to_string(),
                verification_status_id: v.status_id,
                status_tag_color,
                verification_parent_id: v.parent_id,
                verification_parent_title: parent_title,
                verification_parent_reference_code: parent_ref,
                verification_parent_description: parent_desc,
                verification_parent_status_id: parent_status,
                verification_parent_status_variant: parent_status_variant,
                verification_parent_status_tag_color: verification_parent_status_tag_color,
                verification_parent_source: parent_source,
                project_id: v.project_id,
            }
        })
        .collect()
}

/// Retrieve verifications linked to a requirement and return them decorated.
fn get_linked_verifications_for_requirement_impl<R: Repository>(
    repo: &R,
    id: i32,
) -> Result<Vec<DecoratedVerification>, RepoError> {
    let requirement = repo.get_requirement_by_id(id)?;
    let matrix = repo.get_matrix_by_project(requirement.project_id)?;

    let verification_ids: Vec<i32> = matrix
        .into_iter()
        .filter(|m| m.req_id == id)
        .map(|m| m.verification_id)
        .collect();

    if verification_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut verifications = Vec::new();
    for vid in verification_ids {
        verifications.push(repo.get_verification_by_id(vid)?);
    }

    Ok(decorate_verifications_impl(repo, verifications))
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
                is_system: false,
                tag_color: None,
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
                current_version_id: Some(310),
                same_as_current: None,
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
                approval_state: "draft".to_string(),
                approved_by: None,
                approved_at: None,
                custom_fields: None,
            },
        );

        // RequirementVersion entries for link resolution
        use crate::models::RequirementVersion;
        for (vid, rid) in [(10, 1), (20, 2), (30, 3), (310, 31)] {
            repo.requirement_versions.insert(
                vid,
                RequirementVersion {
                    id: vid,
                    requirement_id: rid,
                    title: format!("Version {vid}"),
                    description: String::new(),
                    status_id: 1,
                    author_id: 1,
                    reviewer_id: 2,
                    category_id: 1,
                    created_at: now,
                    deadline_date: None,
                    applicability_id: 1,
                    justification: None,
                    approval_state: "draft".to_string(),
                    approved_by: None,
                    approved_at: None,
                },
            );
        }

        // Links: r2 (vid=20) -> parent 31 (vid=310); r3 (vid=30) -> non-existent parent 32 (vid=320)
        use crate::models::RequirementVersionLink;
        repo.requirement_version_links.push(RequirementVersionLink {
            id: 1,
            source_version_id: 20,
            target_version_id: 310,
            link_type: "DERIVES_FROM".to_string(),
            rationale: None,
            project_id: 1,
            created_at: now,
            metadata: None,
        });
        // Link for r3 to a version (320) whose requirement (32) doesn't exist -> "[Deleted Parent]"
        // We add version 320 pointing to requirement 32 (which is NOT in repo.requirements)
        repo.requirement_versions.insert(
            320,
            RequirementVersion {
                id: 320,
                requirement_id: 32,
                title: "Deleted parent version".into(),
                description: String::new(),
                status_id: 1,
                author_id: 1,
                reviewer_id: 2,
                category_id: 1,
                created_at: now,
                deadline_date: None,
                applicability_id: 1,
                justification: None,
                approval_state: "draft".to_string(),
                approved_by: None,
                approved_at: None,
            },
        );
        repo.requirement_version_links.push(RequirementVersionLink {
            id: 2,
            source_version_id: 30,
            target_version_id: 320,
            link_type: "DERIVES_FROM".to_string(),
            rationale: None,
            project_id: 1,
            created_at: now,
            metadata: None,
        });

        let r1 = Requirement {
            id: 1,
            current_version_id: Some(10),
            same_as_current: None,
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
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        };

        let r2 = Requirement {
            id: 2,
            current_version_id: Some(20),
            same_as_current: None,
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
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        };

        let r3 = Requirement {
            id: 3,
            current_version_id: Some(30),
            same_as_current: None,
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
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
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
        // r3 links to version 320 -> requirement 32, which doesn't exist,
        // so the link is skipped and no parent is found
        assert_eq!(d3.req_parent_title, "");
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
                is_system: false,
                tag_color: None,
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
                is_system: false,
                tag_color: None,
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
                is_system: false,
                tag_color: None,
            },
        );
        let req = Requirement {
            id: 1,
            current_version_id: None,
            same_as_current: None,
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
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
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
            suspect: false,
            suspect_at: None,
            suspect_reason: None,
            cleared_by: None,
            cleared_at: None,
            triggering_version_id: None,
            triggering_user_id: None,
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
            current_version_id: None,
            same_as_current: None,
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
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        };
        repo.requirements.insert(2, req);
        // matrix for different requirement
        repo.matrices.push(MatrixLink {
            req_id: 99,
            test_id: 50,
            creation_date: now,
            project_id: 1,
            suspect: false,
            suspect_at: None,
            suspect_reason: None,
            cleared_by: None,
            cleared_at: None,
            triggering_version_id: None,
            triggering_user_id: None,
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
            current_version_id: None,
            same_as_current: None,
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
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        };
        repo.requirements.insert(3, req);
        repo.matrices.push(MatrixLink {
            req_id: 3,
            test_id: 999,
            creation_date: now,
            project_id: 1,
            suspect: false,
            suspect_at: None,
            suspect_reason: None,
            cleared_by: None,
            cleared_at: None,
            triggering_version_id: None,
            triggering_user_id: None,
        });

        assert!(get_linked_tests_for_requirement_impl(&repo, 3).is_err());
    }
}
