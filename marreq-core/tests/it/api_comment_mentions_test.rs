// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! `@username` mentions in requirement comments (issue #389): project members
//! get a `mentioned` notification, anyone else is ignored, and nobody gets two
//! notifications for one comment.

#![cfg(feature = "test-helpers")]

use marreq_core::models::*;
use marreq_core::permissions::ROLE_AUTHOR;
use marreq_core::repository::diesel_repo_mock::DieselRepoMock;
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use serde_json::json;

use crate::api_project_bundle_test::test_support::{
    TestAppState, bundle_repo, session_cookie, test_client, timestamp,
};

/// Site administrator; author and reviewer of requirement 1.
const ADMIN: i32 = 1;
const VIEWER: i32 = 2;
/// Has an account but is not a member of project 1.
const OUTSIDER: i32 = 3;
const AUTHOR: i32 = 5;

fn repo() -> DieselRepoMock {
    let mut repo = bundle_repo();
    repo.users.insert(
        AUTHOR,
        DieselRepoMock::make_user(AUTHOR, "author", "password"),
    );
    repo.project_members.push(ProjectMember {
        project_id: 1,
        user_id: AUTHOR,
        role: ROLE_AUTHOR,
        created_at: timestamp(),
        updated_at: timestamp(),
    });
    repo
}

async fn comment(client: &Client, path: &str, body: &str) {
    let response = client
        .post(path.to_string())
        .private_cookie(session_cookie(client, AUTHOR))
        .header(ContentType::JSON)
        .body(json!({ "body": body }).to_string())
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Created, "{path}");
}

/// `(user_id, notification_type)` of every notification, in creation order.
fn notifications(client: &Client) -> Vec<(i32, String)> {
    let state = client.rocket().state::<TestAppState>().unwrap();
    let repo = state.repo.read().unwrap();
    repo.inner_repo()
        .notifications
        .iter()
        .map(|n| (n.user_id, n.notification_type.clone()))
        .collect()
}

#[rocket::async_test]
async fn mentions_notify_members_only_and_replace_the_comment_notification() {
    for path in [
        "/api/projects/1/requirements/1/comments",
        "/api/requirements/1/comments",
    ] {
        let client = test_client(repo()).await;
        comment(
            &client,
            path,
            "@Viewer @admin, can you check? cc @outsider @nobody @author viewer@example.com",
        )
        .await;

        let notes = notifications(&client);
        assert_eq!(
            notes,
            vec![(VIEWER, "mentioned".into()), (ADMIN, "mentioned".into())],
            "{path}: the requirement's author is mentioned, so gets no comment_added"
        );
        let state = client.rocket().state::<TestAppState>().unwrap();
        let repo = state.repo.read().unwrap();
        let note = &repo.inner_repo().notifications[0];
        assert!(
            note.title.ends_with("mentioned you on SAT-PWR-001"),
            "{}",
            note.title
        );
        assert_eq!(note.entity_type.as_deref(), Some("requirement"));
        assert_eq!(note.entity_id, Some(1));
        assert!(
            !notes
                .iter()
                .any(|(user, _)| *user == OUTSIDER || *user == AUTHOR)
        );
    }
}

#[rocket::async_test]
async fn a_comment_without_mentions_notifies_the_author_as_before() {
    let client = test_client(repo()).await;
    comment(
        &client,
        "/api/projects/1/requirements/1/comments",
        "Looks good",
    )
    .await;
    assert_eq!(
        notifications(&client),
        vec![(ADMIN, "comment_added".into())]
    );
}
