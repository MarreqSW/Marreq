// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Verification control data and requirement close-out (issue #353).

use std::collections::HashMap;

use crate::api::prelude::*;
use crate::auth::guards::{
    ProjectRequirementsRead, ProjectRequirementsWrite, ProjectVerificationsRead,
    ProjectVerificationsWrite,
};
use crate::services::verification_control_service::{
    ComplianceInput, RequirementCloseOutView, VerificationControlInput, VerificationControlService,
    VerificationControlView,
};

/// Level, stage and evidence of a verification, with suggestions.
#[get("/projects/<project_id>/verifications/<verification_id>/control")]
pub async fn get_verification_control(
    access: ProjectVerificationsRead,
    project_id: i32,
    verification_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<VerificationControlView>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    Ok(Json(
        VerificationControlService::new(state.inner())
            .get_verification_control(project_id, verification_id)?,
    ))
}

/// Replace the level, stage and evidence of a verification.
#[put(
    "/projects/<project_id>/verifications/<verification_id>/control",
    data = "<body>"
)]
pub async fn put_verification_control(
    access: ProjectVerificationsWrite,
    project_id: i32,
    verification_id: i32,
    body: Json<VerificationControlInput>,
    state: &State<AppState>,
) -> ApiResult<Json<VerificationControlView>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;
    Ok(Json(
        VerificationControlService::new(state.inner()).set_verification_control(
            access.user(),
            project_id,
            verification_id,
            body.into_inner(),
        )?,
    ))
}

/// A requirement's compliance assessment, linked verifications and close-out.
#[get("/projects/<project_id>/requirements/<requirement_id>/close_out")]
pub async fn get_requirement_close_out(
    access: ProjectRequirementsRead,
    project_id: i32,
    requirement_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<RequirementCloseOutView>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    Ok(Json(
        VerificationControlService::new(state.inner())
            .requirement_close_out(project_id, requirement_id)?,
    ))
}

/// Close-out of every requirement in the project, keyed by requirement id.
#[get("/projects/<project_id>/close_out")]
pub async fn get_project_close_out(
    access: ProjectRequirementsRead,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<HashMap<i32, RequirementCloseOutView>>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    Ok(Json(
        VerificationControlService::new(state.inner()).project_close_out(project_id)?,
    ))
}

/// Set (`C`, `PC`, `NC`) or clear (`null`) the compliance assessment. Only
/// project reviewers, like changing a verification status.
#[put(
    "/projects/<project_id>/requirements/<requirement_id>/compliance",
    data = "<body>"
)]
pub async fn put_requirement_compliance(
    access: ProjectRequirementsWrite,
    project_id: i32,
    requirement_id: i32,
    body: Json<ComplianceInput>,
    state: &State<AppState>,
) -> ApiResult<Json<RequirementCloseOutView>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::EditRequirements,
    )?;
    require_project_reviewer(state, access.user(), project_id)?;
    Ok(Json(
        VerificationControlService::new(state.inner()).set_requirement_compliance(
            access.user(),
            project_id,
            requirement_id,
            body.into_inner(),
        )?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::session::test_session_cookie_for;
    use crate::models::{
        MatrixLink, Project, ProjectMember, Requirement, Verification, VerificationStatus,
    };
    use crate::permissions::{ROLE_AUTHOR, ROLE_VIEWER};
    use crate::repository::{CacheRepository, diesel_repo_mock::DieselRepoMock};
    use crate::status_enums::ProjectStatus;
    use chrono::{NaiveDate, NaiveDateTime};
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::{Value, json};
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const PROJECT: i32 = 5;
    const OTHER_PROJECT: i32 = 6;
    const REVIEWER: i32 = 1;
    const AUTHOR: i32 = 2;
    const VIEWER: i32 = 3;
    const OUTSIDER: i32 = 4;

    fn ts() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn project(id: i32) -> Project {
        Project {
            id,
            name: format!("Project {id}"),
            description: None,
            creation_date: None,
            update_date: None,
            status: ProjectStatus::Active,
            owner_id: Some(REVIEWER),
            slug: format!("project-{id}"),
            group_id: None,
            archived_at: None,
            archived_by: None,
        }
    }

    fn requirement(id: i32, project_id: i32, code: &str) -> Requirement {
        Requirement {
            id,
            current_version_id: Some(id),
            same_as_current: None,
            title: format!("{code} title"),
            description: "The satellite shall comply.".into(),
            status_id: 1,
            author_id: AUTHOR,
            reviewer_id: REVIEWER,
            reference_code: code.into(),
            category_id: 1,
            parent_id: None,
            creation_date: ts(),
            update_date: ts(),
            deadline_date: None,
            applicability_id: 1,
            justification: None,
            project_id,
            approval_state: "draft".into(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        }
    }

    fn verification(id: i32, project_id: i32, code: &str, status_id: i32) -> Verification {
        Verification {
            id,
            name: format!("{code} name"),
            reference_code: code.into(),
            description: String::new(),
            source: String::new(),
            status_id,
            parent_id: None,
            project_id,
            verification_method_id: None,
            author_id: AUTHOR,
            reviewer_id: REVIEWER,
            status_set_by: None,
            status_set_at: None,
        }
    }

    fn status(id: i32, title: &str, outcome: &str) -> VerificationStatus {
        VerificationStatus {
            id,
            title: title.into(),
            description: String::new(),
            tag: title[..1].into(),
            project_id: PROJECT,
            is_system: true,
            tag_color: None,
            outcome: outcome.into(),
        }
    }

    fn link(req_id: i32, verification_id: i32, project_id: i32) -> MatrixLink {
        MatrixLink {
            req_id,
            verification_id,
            creation_date: ts(),
            project_id,
            suspect: false,
            suspect_at: None,
            suspect_reason: None,
            cleared_by: None,
            cleared_at: None,
            triggering_version_id: None,
            triggering_user_id: None,
        }
    }

    fn repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();
        for (id, name) in [
            (REVIEWER, "rita"),
            (AUTHOR, "arno"),
            (VIEWER, "vera"),
            (OUTSIDER, "otto"),
        ] {
            repo.users
                .insert(id, DieselRepoMock::make_user(id, name, ""));
        }
        repo.projects.insert(PROJECT, project(PROJECT));
        repo.projects.insert(OTHER_PROJECT, project(OTHER_PROJECT));
        for (user_id, role) in [
            (REVIEWER, ROLE_AUTHOR),
            (AUTHOR, ROLE_AUTHOR),
            (VIEWER, ROLE_VIEWER),
        ] {
            repo.project_members.push(ProjectMember {
                project_id: PROJECT,
                user_id,
                role,
                created_at: ts(),
                updated_at: ts(),
            });
        }
        repo.project_reviewers.insert(PROJECT, vec![REVIEWER]);
        repo.verification_statuses
            .insert(10, status(10, "Passed", "passed"));
        repo.verification_statuses
            .insert(11, status(11, "Pending", "not_run"));
        repo.requirements
            .insert(1, requirement(1, PROJECT, "REQ-1"));
        repo.requirements
            .insert(2, requirement(2, PROJECT, "REQ-2"));
        repo.requirements
            .insert(9, requirement(9, OTHER_PROJECT, "OTHER-1"));
        repo.verifications
            .insert(20, verification(20, PROJECT, "VER-1", 10));
        repo.verifications
            .insert(21, verification(21, PROJECT, "VER-2", 11));
        repo.verifications
            .insert(29, verification(29, OTHER_PROJECT, "OTHER-V", 10));
        repo.matrices.push(link(1, 20, PROJECT));
        repo.matrices.push(link(2, 21, PROJECT));
        repo
    }

    async fn client() -> Client {
        let state = AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo(), 0))),
        };
        Client::tracked(rocket::build().manage(state).mount(
            "/api",
            routes![
                get_verification_control,
                put_verification_control,
                get_requirement_close_out,
                get_project_close_out,
                put_requirement_compliance
            ],
        ))
        .await
        .expect("client")
    }

    async fn send(
        client: &Client,
        user: Option<i32>,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> (Status, Value) {
        let path = format!("/api/projects/{path}");
        let mut req = match method {
            "PUT" => client.put(path),
            _ => client.get(path),
        };
        if let Some(body) = body {
            req = req.header(ContentType::JSON).body(body.to_string());
        }
        if let Some(user) = user {
            let state = client.rocket().state::<TestState>().unwrap();
            req = req.private_cookie(test_session_cookie_for(state, user));
        }
        let res = req.dispatch().await;
        let status = res.status();
        let body = res.into_json::<Value>().await.unwrap_or(Value::Null);
        (status, body)
    }

    fn logs(client: &Client) -> Vec<crate::models::Log> {
        let state = client.rocket().state::<TestState>().unwrap();
        state.repo_read().inner_repo().logs.clone()
    }

    #[rocket::async_test]
    async fn reviewers_assess_compliance_and_close_out_follows() {
        let client = client().await;

        let (status, body) = send(
            &client,
            Some(VIEWER),
            "GET",
            "5/requirements/1/close_out",
            None,
        )
        .await;
        assert_eq!(status, Status::Ok);
        assert_eq!(
            body["close_out"],
            json!({"status": "open", "reason": "Compliance not assessed"})
        );
        assert_eq!(body["verifications"][0]["reference_code"], "VER-1");
        assert_eq!(body["verifications"][0]["outcome"], "passed");

        let (status, body) = send(
            &client,
            Some(REVIEWER),
            "PUT",
            "5/requirements/1/compliance",
            Some(json!({"compliance": "C", "note": "  TR-001 accepted "})),
        )
        .await;
        assert_eq!(status, Status::Ok);
        assert_eq!(body["compliance"], "C");
        assert_eq!(body["note"], "TR-001 accepted");
        assert_eq!(
            body["close_out"],
            json!({"status": "closed", "reason": "Accepted (C)"})
        );
        let log = logs(&client).pop().expect("audit entry");
        assert_eq!(log.entity_type, "REQUIREMENT");
        assert_eq!(log.entity_id, Some(1));
        assert_eq!(
            log.description.as_deref(),
            Some("Set compliance of REQ-1 to C")
        );

        let (status, body) = send(&client, Some(VIEWER), "GET", "5/close_out", None).await;
        assert_eq!(status, Status::Ok);
        assert_eq!(body["1"]["close_out"]["status"], "closed");
        assert_eq!(
            body["2"]["close_out"],
            json!({"status": "open", "reason": "VER-2 not run"})
        );
        assert!(body.get("9").is_none(), "other projects are not included");

        let (status, body) = send(
            &client,
            Some(REVIEWER),
            "PUT",
            "5/requirements/1/compliance",
            Some(json!({"compliance": null})),
        )
        .await;
        assert_eq!(status, Status::Ok);
        assert_eq!(body["compliance"], Value::Null);
        assert_eq!(body["close_out"]["reason"], "Compliance not assessed");
    }

    #[rocket::async_test]
    async fn only_reviewers_may_set_compliance_and_input_is_checked() {
        let client = client().await;
        let set = |c: &'static str| Some(json!({ "compliance": c }));
        for (user, expected) in [
            (Some(AUTHOR), Status::Forbidden),
            (Some(VIEWER), Status::Forbidden),
            (None, Status::Unauthorized),
        ] {
            let (status, _) = send(
                &client,
                user,
                "PUT",
                "5/requirements/1/compliance",
                set("C"),
            )
            .await;
            assert_eq!(status, expected, "user {user:?}");
        }
        let (status, _) = send(
            &client,
            Some(OUTSIDER),
            "PUT",
            "5/requirements/1/compliance",
            set("C"),
        )
        .await;
        assert!(
            status == Status::Forbidden || status == Status::NotFound,
            "{status}"
        );
        let (status, _) = send(
            &client,
            Some(REVIEWER),
            "PUT",
            "5/requirements/1/compliance",
            set("Y"),
        )
        .await;
        assert_eq!(status, Status::BadRequest);
        let (status, _) = send(
            &client,
            Some(REVIEWER),
            "PUT",
            "5/requirements/9/compliance",
            set("C"),
        )
        .await;
        assert_eq!(status, Status::NotFound, "requirement of another project");
        let (status, _) = send(
            &client,
            Some(REVIEWER),
            "PUT",
            "5/requirements/1/compliance",
            Some(json!({"compliance": "C", "extra": 1})),
        )
        .await;
        assert_eq!(status, Status::UnprocessableEntity);
        assert!(logs(&client).is_empty(), "rejected requests are not logged");
    }

    #[rocket::async_test]
    async fn editors_record_level_stage_and_evidence() {
        let client = client().await;
        let (status, body) = send(
            &client,
            Some(VIEWER),
            "GET",
            "5/verifications/20/control",
            None,
        )
        .await;
        assert_eq!(status, Status::Ok);
        assert_eq!(body["verification_level"], Value::Null);

        let (status, body) = send(
            &client,
            Some(AUTHOR),
            "PUT",
            "5/verifications/20/control",
            Some(json!({"verification_level": " Subsystem ", "verification_stage": "QUAL", "evidence_reference": "TR-PWR-002"})),
        )
        .await;
        assert_eq!(status, Status::Ok);
        assert_eq!(body["verification_level"], "Subsystem");
        assert_eq!(body["verification_stage"], "QUAL");
        assert_eq!(body["evidence_reference"], "TR-PWR-002");
        assert_eq!(body["updated_by"], AUTHOR);
        assert_eq!(
            body["suggestions"],
            json!({"levels": ["Subsystem"], "stages": ["QUAL"]})
        );
        let log = logs(&client).pop().expect("audit entry");
        assert_eq!(log.entity_type, "VERIFICATION");
        assert_eq!(log.entity_id, Some(20));

        // Empty strings clear a field.
        let (_, body) = send(
            &client,
            Some(AUTHOR),
            "PUT",
            "5/verifications/20/control",
            Some(json!({"verification_level": "", "verification_stage": "QUAL"})),
        )
        .await;
        assert_eq!(body["verification_level"], Value::Null);
        assert_eq!(body["evidence_reference"], Value::Null);

        let long = "x".repeat(41);
        let (status, _) = send(
            &client,
            Some(AUTHOR),
            "PUT",
            "5/verifications/20/control",
            Some(json!({ "verification_stage": long })),
        )
        .await;
        assert_eq!(status, Status::BadRequest);
        let (status, _) = send(
            &client,
            Some(VIEWER),
            "PUT",
            "5/verifications/20/control",
            Some(json!({})),
        )
        .await;
        assert_eq!(status, Status::Forbidden);
        let (status, _) = send(
            &client,
            Some(AUTHOR),
            "PUT",
            "5/verifications/29/control",
            Some(json!({})),
        )
        .await;
        assert_eq!(status, Status::NotFound, "verification of another project");
        let (status, _) = send(
            &client,
            Some(AUTHOR),
            "GET",
            "5/verifications/29/control",
            None,
        )
        .await;
        assert_eq!(status, Status::NotFound);
    }
}
