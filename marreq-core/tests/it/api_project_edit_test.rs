// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Editing project properties (issue #289) through the full API router.

#![cfg(feature = "test-helpers")]

use chrono::{NaiveDate, NaiveDateTime};
use marreq_core::app::AppState;
use marreq_core::auth::session::test_session_cookie_for;
use marreq_core::models::*;
use marreq_core::permissions::{ROLE_ADMIN, ROLE_VIEWER};
use marreq_core::repository::{CacheRepository, diesel_repo_mock::DieselRepoMock};
use marreq_core::status_enums::ProjectStatus;
use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::Client;
use serde_json::{Value, json};
use std::sync::{Arc, RwLock};

type TestAppState = AppState<CacheRepository<DieselRepoMock>>;

fn timestamp() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

fn repo() -> DieselRepoMock {
    let mut repo = DieselRepoMock::default();
    repo.users
        .insert(1, DieselRepoMock::make_user(1, "lead", "password"));
    repo.users
        .insert(2, DieselRepoMock::make_user(2, "viewer", "password"));
    repo.projects.insert(
        3,
        Project {
            id: 3,
            name: "Lunar Lander".into(),
            description: None,
            creation_date: Some(timestamp()),
            update_date: Some(timestamp()),
            status: ProjectStatus::Active,
            owner_id: Some(1),
            slug: "lunar-lander".into(),
            group_id: None,
        },
    );
    for (user_id, role) in [(1, ROLE_ADMIN), (2, ROLE_VIEWER)] {
        repo.project_members.push(ProjectMember {
            project_id: 3,
            user_id,
            role,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
    }
    repo
}

async fn client() -> Client {
    marreq_core::deployment::install_test_server_mode();
    let state: TestAppState = AppState {
        repo: Arc::new(RwLock::new(CacheRepository::new(repo(), 0))),
    };
    let rocket = rocket::build()
        .manage(state)
        .manage(marreq_core::auth::AuthConfig::default())
        .manage(marreq_core::auth::rate_limiter::LoginRateLimiter::new())
        .mount("/api", marreq_core::api::routes());
    Client::tracked(rocket).await.expect("rocket instance")
}

fn session(client: &Client, user_id: i32) -> Cookie<'static> {
    test_session_cookie_for(
        client.rocket().state::<TestAppState>().expect("state"),
        user_id,
    )
}

#[rocket::async_test]
async fn project_admin_edits_properties_and_the_list_reflects_them() {
    let client = client().await;
    let res = client
        .patch("/api/projects/3")
        .header(ContentType::JSON)
        .private_cookie(session(&client, 1))
        .body(
            json!({ "name": "Lunar Lander Mk II", "description": "Descent stage", "status": "Completed" })
                .to_string(),
        )
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);

    let list: Value = client
        .get("/api/projects")
        .private_cookie(session(&client, 1))
        .dispatch()
        .await
        .into_json()
        .await
        .expect("json");
    let project = &list.as_array().unwrap()[0];
    assert_eq!(project["name"], "Lunar Lander Mk II");
    assert_eq!(project["description"], "Descent stage");
    assert_eq!(project["status"], "Completed");
    assert_eq!(project["slug"], "lunar-lander");
}

#[rocket::async_test]
async fn viewer_cannot_edit_project_properties() {
    let client = client().await;
    let res = client
        .patch("/api/projects/3")
        .header(ContentType::JSON)
        .private_cookie(session(&client, 2))
        .body(json!({ "name": "Nope" }).to_string())
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Forbidden);
}
