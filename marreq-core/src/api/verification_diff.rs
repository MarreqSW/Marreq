// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! API endpoints for verification snapshot diffs (read-only, reconstructed from the audit log).

use crate::api::prelude::*;
use crate::auth::guards::ProjectAccessOrBearer;
use crate::diff::VerificationVersionDiff;
use crate::services::verification_diff_service::{VerificationDiffService, VerificationSnapshot};

/// Chronological verification snapshots reconstructed from the audit log.
#[get("/projects/<project_id>/verifications/<id>/snapshots")]
pub async fn list_snapshots_by_project(
    access: ProjectAccessOrBearer,
    project_id: i32,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Vec<VerificationSnapshot>>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = VerificationDiffService::new(state.inner());
    Ok(Json(service.list_snapshots(project_id, id)?))
}

/// Diff two verification snapshots (v1 = old, v2 = new). Both ids must belong to the verification.
#[get("/projects/<project_id>/verifications/<id>/snapshots/<v1>/diff/<v2>")]
pub async fn diff_snapshots_by_project(
    access: ProjectAccessOrBearer,
    project_id: i32,
    id: i32,
    v1: i32,
    v2: i32,
    state: &State<AppState>,
) -> ApiResult<Json<VerificationVersionDiff>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = VerificationDiffService::new(state.inner());
    Ok(Json(service.diff_snapshots(project_id, id, v1, v2)?))
}

/// Diff the verification snapshot stored in a baseline against the current verification.
#[get("/projects/<project_id>/baselines/<baseline_id>/verifications/<id>/diff/current")]
pub async fn diff_baseline_vs_current(
    access: ProjectAccessOrBearer,
    project_id: i32,
    baseline_id: i32,
    id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<VerificationVersionDiff>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let service = VerificationDiffService::new(state.inner());
    Ok(Json(service.diff_baseline_vs_current(
        project_id,
        baseline_id,
        id,
    )?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::test_session_cookie_for;
    use crate::models::{Log, Verification, VerificationMethod, VerificationStatus};
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use chrono::{NaiveDate, NaiveDateTime};
    use rocket::local::asynchronous::Client;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;
    const ADMIN_ID: i32 = 1;

    fn epoch() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn state_from_repo(repo: DieselRepoMock) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(state_from_repo(repo.with_admin_user()))
            .mount(
                "/api",
                routes![list_snapshots_by_project, diff_snapshots_by_project],
            );
        Client::tracked(rocket).await.unwrap()
    }

    fn auth_cookie(client: &rocket::local::asynchronous::Client) -> rocket::http::Cookie<'static> {
        let state = client.rocket().state::<TestState>().unwrap();
        test_session_cookie_for(state, ADMIN_ID)
    }

    fn seeded_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();
        repo.verifications.insert(
            1,
            Verification {
                id: 1,
                name: "Power test".into(),
                reference_code: "VER-001".into(),
                description: "Measure 650W".into(),
                source: "TV-001".into(),
                status_id: 2,
                parent_id: None,
                project_id: 1,
                verification_method_id: Some(1),
                author_id: 1,
                reviewer_id: 1,
                status_set_by: None,
                status_set_at: None,
            },
        );
        repo.verification_statuses.insert(
            1,
            VerificationStatus {
                id: 1,
                title: "Not run".into(),
                description: String::new(),
                tag: "NR".into(),
                project_id: 1,
                is_system: true,
                tag_color: None,
            },
        );
        repo.verification_statuses.insert(
            2,
            VerificationStatus {
                id: 2,
                title: "Passed".into(),
                description: String::new(),
                tag: "P".into(),
                project_id: 1,
                is_system: true,
                tag_color: None,
            },
        );
        repo.verification_methods.insert(
            1,
            VerificationMethod {
                id: 1,
                title: "Test".into(),
                description: String::new(),
                tag: "T".into(),
                project_id: 1,
            },
        );
        let v1 = r#"{"name":"Power test","description":"Measure 500W","source":"TV-001","reference_code":"VER-001","status_id":1,"parent_id":null,"verification_method_id":1}"#;
        let v2 = r#"{"name":"Power test","description":"Measure 650W","source":"TV-001","reference_code":"VER-001","status_id":2,"parent_id":null,"verification_method_id":1}"#;
        repo.logs.push(Log {
            log_id: 10,
            user_id: 1,
            action_type: "CREATE".into(),
            entity_type: "VERIFICATION".into(),
            entity_id: Some(1),
            project_id: Some(1),
            old_values: None,
            new_values: Some(v1.into()),
            description: None,
            ip_address: None,
            user_agent: None,
            created_at: epoch(),
        });
        repo.logs.push(Log {
            log_id: 11,
            user_id: 1,
            action_type: "UPDATE".into(),
            entity_type: "VERIFICATION".into(),
            entity_id: Some(1),
            project_id: Some(1),
            old_values: Some(v1.into()),
            new_values: Some(v2.into()),
            description: None,
            ip_address: None,
            user_agent: None,
            created_at: epoch() + chrono::Duration::seconds(1),
        });
        repo
    }

    #[rocket::async_test]
    async fn list_and_diff_snapshots() {
        let client = client_with_repo(seeded_repo()).await;
        let list_resp = client
            .get("/api/projects/1/verifications/1/snapshots")
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(list_resp.status(), Status::Ok);
        let snapshots: Vec<VerificationSnapshot> = list_resp.into_json().await.unwrap();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].id, 10);
        assert_eq!(snapshots[1].id, 11);

        let diff_resp = client
            .get("/api/projects/1/verifications/1/snapshots/10/diff/11")
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(diff_resp.status(), Status::Ok);
        let diff: VerificationVersionDiff = diff_resp.into_json().await.unwrap();
        assert_eq!(diff.text.description.removed, ["Measure 500W"]);
        assert_eq!(diff.text.description.added, ["Measure 650W"]);
        assert_eq!(diff.metadata.status.old_label.as_deref(), Some("Not run"));
        assert_eq!(diff.metadata.status.new_label.as_deref(), Some("Passed"));
        assert_eq!(
            diff.metadata.verification_method.unchanged_label.as_deref(),
            Some("Test")
        );

        let wrong_project = client
            .get("/api/projects/2/verifications/1/snapshots/10/diff/11")
            .private_cookie(auth_cookie(&client))
            .dispatch()
            .await;
        assert_eq!(wrong_project.status(), Status::NotFound);
    }
}
