// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Archived projects (issue #381): read-only for everyone, site administrators
//! included, still readable and exportable, listed with `archived: true`, and
//! restorable by the owner or an instance administrator.

#![cfg(feature = "test-helpers")]

use marreq_core::models::*;
use marreq_core::permissions::{ROLE_ADMIN, ROLE_AUTHOR};
use marreq_core::repository::diesel_repo_mock::DieselRepoMock;
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::{Client, LocalResponse};
use serde_json::{Value, json};

use crate::api_project_bundle_test::test_support::{
    TestAppState, bundle_repo, session_cookie, test_client, timestamp,
};

/// Site administrator and owner of project 1.
const OWNER_ADMIN: i32 = 1;
/// The project's Admin role, but not its owner.
const PROJECT_ADMIN: i32 = 4;
const AUTHOR: i32 = 5;
const OUTSIDER: i32 = 3;

const ARCHIVED: &str = "This project is archived; unarchive it to make changes.";

fn repo() -> DieselRepoMock {
    let mut repo = bundle_repo();
    for (id, name, role) in [
        (PROJECT_ADMIN, "lead", ROLE_ADMIN),
        (AUTHOR, "author", ROLE_AUTHOR),
    ] {
        repo.users
            .insert(id, DieselRepoMock::make_user(id, name, "password"));
        repo.project_members.push(ProjectMember {
            project_id: 1,
            user_id: id,
            role,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
    }
    repo
}

async fn call<'c>(
    client: &'c Client,
    user: i32,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> LocalResponse<'c> {
    let path = path.to_string();
    let request = match method {
        "GET" => client.get(path),
        "POST" => client.post(path),
        "PUT" => client.put(path),
        "PATCH" => client.patch(path),
        "DELETE" => client.delete(path),
        other => panic!("unsupported method {other}"),
    }
    .private_cookie(session_cookie(client, user));
    match body {
        Some(body) => {
            request
                .header(ContentType::JSON)
                .body(body.to_string())
                .dispatch()
                .await
        }
        None => request.dispatch().await,
    }
}

async fn archive(client: &Client, user: i32) -> Status {
    call(client, user, "POST", "/api/projects/1/archive", None)
        .await
        .status()
}

fn project(client: &Client) -> Project {
    let state = client.rocket().state::<TestAppState>().unwrap();
    let repo = state.repo.read().unwrap();
    repo.inner_repo().projects[&1].clone()
}

/// One request per kind of change to project 1.
fn writes() -> Vec<(&'static str, &'static str, Option<Value>)> {
    vec![
        (
            "PATCH",
            "/api/projects/1/requirements/1",
            Some(json!({ "title": "Changed" })),
        ),
        (
            "PUT",
            "/api/projects/1/requirements/1/versions/10/approval",
            Some(json!({ "state": "reviewed" })),
        ),
        (
            "POST",
            "/api/projects/1/requirements/1/comments",
            Some(json!({ "body": "A remark" })),
        ),
        (
            "POST",
            "/api/categories",
            Some(json!({ "title": "Thermal", "description": "d", "tag": "THM", "project_id": 1 })),
        ),
        (
            "POST",
            "/api/projects/1/saved_views",
            Some(json!({ "name": "Mine", "visibility": "private", "definition": {} })),
        ),
        (
            "POST",
            "/api/projects/1/report_templates",
            Some(json!({ "name": "Template" })),
        ),
        (
            "PUT",
            "/api/projects/1/members/2",
            Some(json!({ "role": 3 })),
        ),
        (
            "PATCH",
            "/api/projects/1",
            Some(json!({ "name": "Renamed", "description": null })),
        ),
    ]
}

#[rocket::async_test]
async fn only_the_owner_or_an_instance_admin_may_archive() {
    let client = test_client(repo()).await;
    assert_eq!(archive(&client, PROJECT_ADMIN).await, Status::Forbidden);
    assert_eq!(archive(&client, AUTHOR).await, Status::Forbidden);
    assert!(!project(&client).is_archived());

    let response = call(
        &client,
        OWNER_ADMIN,
        "POST",
        "/api/projects/1/archive",
        None,
    )
    .await;
    assert_eq!(response.status(), Status::Ok);
    let body: Value = response.into_json().await.unwrap();
    assert!(body["archived_at"].is_string());
    assert_eq!(body["archived_by"], OWNER_ADMIN);
    assert_eq!(
        archive(&client, OWNER_ADMIN).await,
        Status::Conflict,
        "already archived"
    );

    let state = client.rocket().state::<TestAppState>().unwrap();
    let logs: Vec<String> = state
        .repo
        .read()
        .unwrap()
        .inner_repo()
        .logs
        .iter()
        .map(|l| l.action_type.clone())
        .collect();
    assert!(logs.contains(&"ARCHIVE".to_string()), "{logs:?}");
}

#[rocket::async_test]
async fn an_archived_project_refuses_every_change_even_for_site_admins() {
    let client = test_client(repo()).await;
    assert_eq!(archive(&client, OWNER_ADMIN).await, Status::Ok);

    for user in [OWNER_ADMIN, PROJECT_ADMIN, AUTHOR] {
        for (method, path, body) in writes() {
            let response = call(&client, user, method, path, body).await;
            assert_eq!(
                response.status(),
                Status::Forbidden,
                "user {user}: {method} {path}"
            );
            let message: Value = response.into_json().await.unwrap_or(Value::Null);
            assert_eq!(message["message"], ARCHIVED, "user {user}: {method} {path}");
        }
    }
    let state = client.rocket().state::<TestAppState>().unwrap();
    let repo = state.repo.read().unwrap();
    assert_eq!(repo.inner_repo().requirements[&1].title, "Bus voltage");
    assert_eq!(repo.inner_repo().projects[&1].name, "Satellite Demo");
}

#[rocket::async_test]
async fn an_archived_project_stays_readable_and_exportable() {
    let client = test_client(repo()).await;
    assert_eq!(archive(&client, OWNER_ADMIN).await, Status::Ok);

    for path in [
        "/api/projects/1/requirements",
        "/api/projects/1/requirements/1",
        "/api/projects/1/exports/bundle.json",
        "/api/requirements/1/comments",
    ] {
        let status = call(&client, AUTHOR, "GET", path, None).await.status();
        assert_eq!(status, Status::Ok, "{path}");
    }

    let perms: Value = call(
        &client,
        AUTHOR,
        "GET",
        "/api/projects/1/me/permissions",
        None,
    )
    .await
    .into_json()
    .await
    .unwrap();
    assert_eq!(perms["archived"], true);
    assert_eq!(perms["view_requirements"], true);
    for flag in [
        "edit_requirements",
        "is_project_reviewer",
        "manage_project_configuration",
        "manage_project_members",
    ] {
        assert_eq!(perms[flag], false, "{flag}");
    }

    // Lists keep the project, marked archived.
    let dashboard: Value = call(&client, AUTHOR, "GET", "/api/dashboard", None)
        .await
        .into_json()
        .await
        .unwrap();
    let listed = dashboard["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["project_id"] == 1)
        .expect("archived project still in the dashboard payload");
    assert_eq!(listed["archived"], true);
}

#[rocket::async_test]
async fn unarchiving_restores_editing_and_delete_still_works() {
    let client = test_client(repo()).await;
    assert_eq!(archive(&client, OWNER_ADMIN).await, Status::Ok);
    let client = &client;
    let unarchive = |user| async move {
        call(client, user, "POST", "/api/projects/1/unarchive", None)
            .await
            .status()
    };
    assert_eq!(unarchive(PROJECT_ADMIN).await, Status::Forbidden);
    assert_eq!(unarchive(OWNER_ADMIN).await, Status::Ok);
    assert!(!project(client).is_archived());
    assert_eq!(
        unarchive(OWNER_ADMIN).await,
        Status::Conflict,
        "not archived"
    );

    let patched = call(
        client,
        AUTHOR,
        "PATCH",
        "/api/projects/1/requirements/1",
        Some(json!({ "title": "Changed" })),
    )
    .await;
    assert_eq!(patched.status(), Status::Ok);

    // An archived project can still be deleted.
    assert_eq!(archive(client, OWNER_ADMIN).await, Status::Ok);
    let deleted = call(
        client,
        OWNER_ADMIN,
        "DELETE",
        "/api/projects/1",
        Some(json!({ "confirm_slug": "satellite-demo" })),
    )
    .await;
    assert_eq!(deleted.status(), Status::NoContent);
}

#[rocket::async_test]
async fn outsiders_cannot_archive_and_are_not_told_about_it() {
    let client = test_client(repo()).await;
    assert_ne!(archive(&client, OUTSIDER).await, Status::Ok);
    assert_eq!(archive(&client, OWNER_ADMIN).await, Status::Ok);
    // An outsider gets the usual refusal, not the archived message.
    let response = call(
        &client,
        OUTSIDER,
        "PATCH",
        "/api/projects/1/requirements/1",
        Some(json!({ "title": "x" })),
    )
    .await;
    let status = response.status();
    assert!(
        status == Status::Forbidden || status == Status::NotFound,
        "{status}"
    );
    let body: Value = response.into_json().await.unwrap_or(Value::Null);
    assert_ne!(body["message"], ARCHIVED);
}
