// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! The start screen (issue #387): `/api/home/attention` and `/api/search`.

#![cfg(feature = "test-helpers")]

use marreq_core::models::*;
use marreq_core::repository::diesel_repo_mock::DieselRepoMock;
use rocket::http::Status;
use rocket::local::asynchronous::Client;
use serde_json::Value;

use crate::api_project_bundle_test::test_support::{
    bundle_repo, session_cookie, test_client, timestamp,
};

/// Site administrator; the project has no reviewer pool, so they review.
const ADMIN: i32 = 1;
const VIEWER: i32 = 2;
const OUTSIDER: i32 = 3;

fn notification(id: i32, user_id: i32, kind: &str, read: bool) -> Notification {
    Notification {
        id,
        user_id,
        project_id: Some(1),
        notification_type: kind.into(),
        title: format!("{kind} #{id}"),
        body: None,
        entity_type: Some("requirement".into()),
        entity_id: Some(1),
        actor_id: Some(VIEWER),
        read,
        emailed: false,
        created_at: timestamp() + chrono::Duration::days(i64::from(id)),
    }
}

/// Requirement 1 waiting for approval, requirement 2 a draft, a suspect
/// link, and notifications of every kind.
fn busy_repo() -> DieselRepoMock {
    let mut repo = bundle_repo();
    repo.requirements.get_mut(&1).unwrap().approval_state = "reviewed".into();
    let mut draft = repo.requirements[&1].clone();
    draft.id = 2;
    draft.reference_code = "SAT-PWR-0010".into();
    draft.title = "Battery depth of discharge".into();
    draft.approval_state = "draft".into();
    repo.requirements.insert(2, draft);
    repo.matrices[0].suspect = true;
    repo.matrices[0].suspect_at = Some(timestamp());
    repo.notifications = vec![
        notification(1, ADMIN, "comment_added", false),
        notification(2, ADMIN, "review_assigned", false),
        notification(3, ADMIN, "requirement_updated", false),
        notification(4, ADMIN, "comment_added", true),
        notification(5, ADMIN, "approval_requested", false),
        notification(6, VIEWER, "comment_added", false),
        notification(7, ADMIN, "mentioned", false),
    ];
    repo
}

async fn get(client: &Client, user: i32, path: &str) -> (Status, Value) {
    let response = client
        .get(path)
        .private_cookie(session_cookie(client, user))
        .dispatch()
        .await;
    let status = response.status();
    (status, response.into_json().await.unwrap_or(Value::Null))
}

fn kinds(body: &Value) -> Vec<String> {
    body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["kind"].as_str().unwrap().to_string())
        .collect()
}

#[rocket::async_test]
async fn attention_lists_approvals_drafts_suspect_links_and_notifications() {
    let client = test_client(busy_repo()).await;
    let (status, body) = get(&client, ADMIN, "/api/home/attention").await;
    assert_eq!(status, Status::Ok);
    assert_eq!(body["approvals"], 1);
    assert_eq!(body["reviews"], 1);
    assert_eq!(body["suspect_links"], 1);
    assert_eq!(
        body["notifications"], 3,
        "unread comment, review_assigned and mentioned only"
    );

    let items = body["items"].as_array().unwrap();
    let approval = items.iter().find(|i| i["kind"] == "approval").unwrap();
    assert_eq!(approval["requirement_id"], 1);
    assert_eq!(
        approval["title"],
        "SAT-PWR-001 is waiting for your approval"
    );
    let review = items.iter().find(|i| i["kind"] == "review").unwrap();
    assert_eq!(review["title"], "1 draft requirement to review");
    let suspect = items.iter().find(|i| i["kind"] == "suspect").unwrap();
    assert_eq!(suspect["count"], 1);
    let ids: Vec<i64> = items
        .iter()
        .filter_map(|i| i["notification_id"].as_i64())
        .collect();
    assert_eq!(
        ids,
        vec![7, 2, 1],
        "newest first; read, other users' and other types left out"
    );
}

#[rocket::async_test]
async fn attention_is_empty_for_a_viewer_and_for_archived_projects() {
    let client = test_client(busy_repo()).await;
    let (_, body) = get(&client, VIEWER, "/api/home/attention").await;
    // Not a reviewer and cannot edit; the one notification is theirs.
    assert_eq!(kinds(&body), vec!["notification"]);

    let mut repo = busy_repo();
    repo.notifications.clear();
    repo.projects.get_mut(&1).unwrap().archived_at = Some(timestamp());
    let client = test_client(repo).await;
    let (_, body) = get(&client, ADMIN, "/api/home/attention").await;
    assert!(kinds(&body).is_empty(), "{body}");
}

#[rocket::async_test]
async fn attention_follows_the_reviewer_pool() {
    let mut repo = busy_repo();
    repo.notifications.clear();
    repo.project_reviewers.insert(1, vec![VIEWER]);
    let client = test_client(repo).await;
    // The admin is not in the pool now: only the suspect link is theirs.
    let (_, body) = get(&client, ADMIN, "/api/home/attention").await;
    assert_eq!(kinds(&body), vec!["suspect"]);
}

#[rocket::async_test]
async fn search_ranks_reference_codes_and_respects_visibility() {
    let client = test_client(busy_repo()).await;
    let codes = |body: &Value| -> Vec<String> {
        body.as_array()
            .unwrap()
            .iter()
            .map(|h| h["reference_code"].as_str().unwrap().to_string())
            .collect()
    };

    let (status, body) = get(&client, VIEWER, "/api/search?q=sat-pwr-001").await;
    assert_eq!(status, Status::Ok);
    assert_eq!(
        codes(&body),
        vec!["SAT-PWR-001", "SAT-PWR-0010"],
        "exact match first"
    );
    assert_eq!(body[0]["project_id"], 1);
    assert_eq!(body[0]["requirement_id"], 1);

    let (_, body) = get(&client, VIEWER, "/api/search?q=battery").await;
    assert_eq!(codes(&body), vec!["SAT-PWR-0010"], "title match");
    let (_, body) = get(&client, VIEWER, "/api/search?q=s").await;
    assert_eq!(body, Value::Array(vec![]), "one character is too short");
    let (_, body) = get(&client, VIEWER, "/api/search?q=sat&limit=1").await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    let (_, body) = get(&client, OUTSIDER, "/api/search?q=sat").await;
    assert_eq!(body, Value::Array(vec![]), "not a member");
}

#[rocket::async_test]
async fn both_routes_need_a_session() {
    let client = test_client(busy_repo()).await;
    for path in ["/api/home/attention", "/api/search?q=sat"] {
        let status = client.get(path).dispatch().await.status();
        assert_eq!(status, Status::Unauthorized, "{path}");
    }
}
