// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Service-boundary authorization of project configuration (issue #288).
//!
//! Every project-owned configuration mutation must check the actor against the
//! project (the stored one for update/delete), refuse to move rows between
//! projects, and let site admins and project admins through.

use std::sync::{Arc, RwLock};

use chrono::{NaiveDate, NaiveDateTime};

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{
    Applicability, Category, CustomFieldDefinition, CustomFieldDefinitionPayload, NewApplicability,
    NewCategory, NewProject, NewRequirementStatus, NewVerificationMethod, NewVerificationStatus,
    ProjectMember, RequirementStatus, User, VerificationMethod, VerificationStatus,
};
use crate::permissions::{ROLE_ADMIN, ROLE_AUTHOR};
use crate::repository::diesel_repo_mock::DieselRepoMock;
use crate::repository::errors::RepoError;
use crate::services::{
    ApplicabilityService, CategoryService, CustomFieldService, ProjectService, StatusService,
    VerificationMethodService,
};

/// Every fixture row lives in project 1; project 2 is the "other" project.
const PROJECT: i32 = 1;
const OTHER: i32 = 2;

const OUTSIDER: i32 = 9;
const AUTHOR: i32 = 5;
const PROJECT_ADMIN: i32 = 6;
const OTHER_PROJECT_ADMIN: i32 = 7;

fn timestamp() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

fn member(project_id: i32, user_id: i32, role: i32) -> ProjectMember {
    ProjectMember {
        project_id,
        user_id,
        role,
        created_at: timestamp(),
        updated_at: timestamp(),
    }
}

fn user(id: i32) -> User {
    DieselRepoMock::make_user(id, &format!("user{id}"), "")
}

fn site_admin() -> User {
    let mut admin = DieselRepoMock::make_user(1, "site-admin", "");
    admin.is_admin = true;
    admin
}

/// One row of each configuration kind (id 1, project 1) plus the members.
fn state() -> AppState<DieselCachedRepo> {
    let mut repo = DieselRepoMock::default();
    repo.project_members.extend([
        member(PROJECT, AUTHOR, ROLE_AUTHOR),
        member(PROJECT, PROJECT_ADMIN, ROLE_ADMIN),
        member(OTHER, OTHER_PROJECT_ADMIN, ROLE_ADMIN),
    ]);
    repo.categories.insert(
        1,
        Category {
            id: 1,
            title: "Cat".into(),
            description: "d".into(),
            tag: "CAT".into(),
            project_id: PROJECT,
        },
    );
    repo.applicability.insert(
        1,
        Applicability {
            id: 1,
            title: "App".into(),
            description: "d".into(),
            tag: "APP".into(),
            project_id: PROJECT,
        },
    );
    repo.requirement_statuses.insert(
        1,
        RequirementStatus {
            id: 1,
            title: "Custom".into(),
            description: "d".into(),
            tag: "CUS".into(),
            project_id: PROJECT,
            is_system: false,
            tag_color: None,
        },
    );
    repo.verification_statuses.insert(
        1,
        VerificationStatus {
            id: 1,
            title: "Custom".into(),
            description: "d".into(),
            tag: "CUS".into(),
            project_id: PROJECT,
            is_system: false,
            tag_color: None,
            outcome: crate::status_enums::default_outcome(),
        },
    );
    repo.verification_methods.insert(
        1,
        VerificationMethod {
            id: 1,
            title: "Test".into(),
            description: "d".into(),
            tag: "TEST".into(),
            project_id: PROJECT,
        },
    );
    repo.custom_field_definitions.insert(
        1,
        CustomFieldDefinition {
            id: 1,
            project_id: PROJECT,
            label: "Owner".into(),
            field_type: "text".into(),
            enum_values: None,
            sort_order: 0,
            created_at: timestamp(),
        },
    );
    repo.next_custom_field_id = 2;
    AppState {
        repo: Arc::new(RwLock::new(DieselCachedRepo::new(repo, 0))),
    }
}

fn category(project_id: i32) -> NewCategory {
    NewCategory {
        id: None,
        title: "Category".into(),
        description: "desc".into(),
        tag: "CATX".into(),
        project_id,
    }
}

fn applicability(project_id: i32) -> NewApplicability {
    NewApplicability {
        id: None,
        title: "Applicability".into(),
        description: "desc".into(),
        tag: "APPX".into(),
        project_id,
    }
}

fn requirement_status(project_id: i32) -> NewRequirementStatus {
    NewRequirementStatus {
        id: None,
        title: "Status".into(),
        description: "desc".into(),
        tag: "STX".into(),
        project_id,
        is_system: false,
        tag_color: None,
    }
}

fn verification_status(project_id: i32) -> NewVerificationStatus {
    NewVerificationStatus {
        id: None,
        title: "Status".into(),
        description: "desc".into(),
        tag: "STX".into(),
        project_id,
        is_system: false,
        tag_color: None,
        outcome: None,
    }
}

fn method(project_id: i32) -> NewVerificationMethod {
    NewVerificationMethod {
        id: None,
        title: "Method".into(),
        description: "desc".into(),
        tag: "MTX".into(),
        project_id,
    }
}

fn custom_field() -> CustomFieldDefinitionPayload {
    CustomFieldDefinitionPayload {
        label: "Field".into(),
        field_type: "text".into(),
        enum_values: None,
        sort_order: None,
    }
}

/// Runs create, update and delete of every configuration kind as `actor`
/// (create in `create_project`, update/delete on the fixture rows in project 1)
/// and returns each result as `Ok(())` or the error.
fn run_all(actor: &User, create_project: i32) -> Vec<(&'static str, Result<(), RepoError>)> {
    let state = state();
    let cat = CategoryService::new(&state);
    let app = ApplicabilityService::new(&state);
    let status = StatusService::new(&state);
    let methods = VerificationMethodService::new(&state);
    let fields = CustomFieldService::new(&state);
    vec![
        (
            "category create",
            cat.create(actor, category(create_project)).map(drop),
        ),
        (
            "category update",
            cat.update(actor, 1, category(PROJECT)).map(drop),
        ),
        ("category delete", cat.delete(actor, 1).map(drop)),
        (
            "applicability create",
            app.create(actor, applicability(create_project)).map(drop),
        ),
        (
            "applicability update",
            app.update(actor, 1, applicability(PROJECT)).map(drop),
        ),
        ("applicability delete", app.delete(actor, 1).map(drop)),
        (
            "requirement status create",
            status
                .create_requirement_status(actor, requirement_status(create_project))
                .map(drop),
        ),
        (
            "requirement status update",
            status
                .update_requirement_status(actor, 1, &requirement_status(PROJECT))
                .map(drop),
        ),
        (
            "requirement status delete",
            status.delete_requirement_status(actor, 1).map(drop),
        ),
        (
            "verification status create",
            status
                .create_verification_status(actor, verification_status(create_project))
                .map(drop),
        ),
        (
            "verification status update",
            status
                .update_verification_status(actor, 1, &verification_status(PROJECT))
                .map(drop),
        ),
        (
            "verification status delete",
            status.delete_verification_status(actor, 1).map(drop),
        ),
        (
            "method create",
            methods.create(actor, method(create_project)).map(drop),
        ),
        (
            "method update",
            methods.update(actor, 1, method(PROJECT)).map(drop),
        ),
        ("method delete", methods.delete(actor, 1).map(drop)),
        (
            "custom field create",
            fields
                .create(actor, create_project, custom_field())
                .map(drop),
        ),
        (
            "custom field update",
            fields.update(actor, 1, custom_field()),
        ),
        ("custom field delete", fields.delete(actor, 1)),
    ]
}

fn assert_all_refused(actor: &User, create_project: i32) {
    for (op, result) in run_all(actor, create_project) {
        assert!(
            matches!(result, Err(RepoError::Unauthorized)),
            "{op}: expected Unauthorized, got {result:?}"
        );
    }
}

fn assert_all_allowed(actor: &User) {
    for (op, result) in run_all(actor, PROJECT) {
        assert!(result.is_ok(), "{op}: expected Ok, got {result:?}");
    }
}

#[test]
fn non_member_is_refused_everything() {
    assert_all_refused(&user(OUTSIDER), PROJECT);
}

#[test]
fn author_member_is_refused_configuration() {
    assert_all_refused(&user(AUTHOR), PROJECT);
}

/// The actor administers project 2 and every payload names project 2 where it
/// can, but the stored rows belong to project 1: update/delete must be checked
/// against the stored project, so all of them fail.
#[test]
fn admin_of_another_project_cannot_touch_this_projects_rows() {
    let state = state();
    let actor = user(OTHER_PROJECT_ADMIN);
    let results: Vec<(&str, Result<(), RepoError>)> = vec![
        (
            "category update",
            CategoryService::new(&state)
                .update(&actor, 1, category(OTHER))
                .map(drop),
        ),
        (
            "applicability update",
            ApplicabilityService::new(&state)
                .update(&actor, 1, applicability(OTHER))
                .map(drop),
        ),
        (
            "requirement status update",
            StatusService::new(&state)
                .update_requirement_status(&actor, 1, &requirement_status(OTHER))
                .map(drop),
        ),
        (
            "verification status update",
            StatusService::new(&state)
                .update_verification_status(&actor, 1, &verification_status(OTHER))
                .map(drop),
        ),
        (
            "method update",
            VerificationMethodService::new(&state)
                .update(&actor, 1, method(OTHER))
                .map(drop),
        ),
        (
            "custom field update",
            CustomFieldService::new(&state).update(&actor, 1, custom_field()),
        ),
    ];
    for (op, result) in results {
        assert!(
            matches!(result, Err(RepoError::Unauthorized)),
            "{op}: expected Unauthorized, got {result:?}"
        );
    }
    // Deletes and creates in project 1 are refused the same way.
    assert_all_refused(&actor, PROJECT);
}

#[test]
fn project_admin_member_may_manage_configuration() {
    assert_all_allowed(&user(PROJECT_ADMIN));
}

#[test]
fn site_admin_may_manage_configuration_without_membership() {
    assert_all_allowed(&site_admin());
}

#[test]
fn updates_cannot_move_rows_to_another_project() {
    let state = state();
    let admin = site_admin();
    let results: Vec<(&str, Result<(), RepoError>)> = vec![
        (
            "category",
            CategoryService::new(&state)
                .update(&admin, 1, category(OTHER))
                .map(drop),
        ),
        (
            "applicability",
            ApplicabilityService::new(&state)
                .update(&admin, 1, applicability(OTHER))
                .map(drop),
        ),
        (
            "requirement status",
            StatusService::new(&state)
                .update_requirement_status(&admin, 1, &requirement_status(OTHER))
                .map(drop),
        ),
        (
            "verification status",
            StatusService::new(&state)
                .update_verification_status(&admin, 1, &verification_status(OTHER))
                .map(drop),
        ),
        (
            "verification method",
            VerificationMethodService::new(&state)
                .update(&admin, 1, method(OTHER))
                .map(drop),
        ),
    ];
    for (kind, result) in results {
        assert!(
            matches!(result, Err(RepoError::CrossProjectViolation(_))),
            "{kind}: expected CrossProjectViolation, got {result:?}"
        );
    }
    let repo = state.repo_read();
    let repo = repo.inner_repo();
    assert_eq!(repo.categories[&1].project_id, PROJECT);
    assert_eq!(repo.applicability[&1].project_id, PROJECT);
    assert_eq!(repo.verification_methods[&1].project_id, PROJECT);
}

/// Project creation seeds its defaults through the trusted bootstrap path, so
/// it works even when the actor does not become a member of the new project.
#[test]
fn project_creation_seeds_defaults_for_a_non_member_actor() {
    // An empty repo: the new project gets id 1, like the fixture rows would.
    let state = AppState {
        repo: Arc::new(RwLock::new(DieselCachedRepo::new(
            DieselRepoMock::default(),
            0,
        ))),
    };
    let actor = user(OUTSIDER);
    let project_id = ProjectService::new(&state)
        .create(
            &actor,
            NewProject {
                name: "Bootstrapped".into(),
                description: None,
                owner_id: Some(AUTHOR),
                status: Default::default(),
                group_id: None,
            },
        )
        .expect("project creation seeds defaults");

    let repo = state.repo_read();
    let repo = repo.inner_repo();
    let in_project = |p: i32| p == project_id;
    assert!(repo.categories.values().any(|c| in_project(c.project_id)));
    assert!(
        repo.applicability
            .values()
            .any(|a| in_project(a.project_id))
    );
    assert_eq!(
        repo.verification_methods
            .values()
            .filter(|m| in_project(m.project_id))
            .count(),
        4
    );
    assert!(
        repo.requirement_statuses
            .values()
            .any(|s| in_project(s.project_id) && s.is_system)
    );
    assert!(
        repo.verification_statuses
            .values()
            .any(|s| in_project(s.project_id) && s.is_system)
    );
}
