// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Use cases over HTTP (issue #398): create, edit the shared draft with
//! operations, publish versions, approve, and the permission, revision and
//! document checks around them.

#![cfg(feature = "test-helpers")]

use marreq_core::models::*;
use marreq_core::permissions::ROLE_AUTHOR;
use marreq_core::repository::diesel_repo_mock::DieselRepoMock;
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::Client;
use serde_json::{Value, json};

use crate::api_project_bundle_test::test_support::{
    bundle_repo, session_cookie, test_client, timestamp,
};

/// Site administrator; may approve (the project has no reviewer list).
const ADMIN: i32 = 1;
const VIEWER: i32 = 2;
const OUTSIDER: i32 = 3;
const AUTHOR: i32 = 5;
const BASE: &str = "/api/projects/1/use-cases";

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

async fn call(
    client: &Client,
    user: i32,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (Status, Value) {
    let path = path.to_string();
    let request = match method {
        "GET" => client.get(path),
        "POST" => client.post(path),
        "PUT" => client.put(path),
        "DELETE" => client.delete(path),
        _ => unreachable!(),
    }
    .private_cookie(session_cookie(client, user));
    let request = match body {
        Some(body) => request.header(ContentType::JSON).body(body.to_string()),
        None => request,
    };
    let response = request.dispatch().await;
    let status = response.status();
    let body = response
        .into_string()
        .await
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Value::Null);
    (status, body)
}

async fn ops(client: &Client, id: i64, base: Value, operations: Value) -> (Status, Value) {
    call(
        client,
        AUTHOR,
        "POST",
        &format!("{BASE}/{id}/draft/operations"),
        Some(json!({ "base_revision": base, "operations": operations })),
    )
    .await
}

/// A use case with three steps and an alternative from step 2; returns its id.
async fn export_use_case(client: &Client) -> i64 {
    let (status, created) = call(
        client,
        AUTHOR,
        "POST",
        BASE,
        Some(json!({ "title": "Export requirements", "primary_actor": "Engineer" })),
    )
    .await;
    assert_eq!(status, Status::Created, "{created}");
    let id = created["id"].as_i64().unwrap();
    let (status, draft) = ops(
        client,
        id,
        json!(1),
        json!([
            { "op": "add_element", "list": "main", "element": { "id": "s_Open01", "text": "User opens the requirements page." } },
            { "op": "add_element", "list": "main", "element": { "id": "s_Pick01", "text": "User selects requirements." } },
            { "op": "add_element", "list": "main", "element": { "id": "s_Make01", "text": "System generates the file." } },
            { "op": "add_branch", "branch": {
                "id": "b_None01", "kind": "alternative", "title": "Nothing selected",
                "from_step": "s_Pick01", "end": { "type": "rejoin", "step": "s_Pick01" },
                "steps": [{ "id": "s_Ask001", "text": "System asks for a selection." }]
            } }
        ]),
    )
    .await;
    assert_eq!(status, Status::Ok, "{draft}");
    assert_eq!(draft["revision"], 2);
    id
}

#[rocket::async_test]
async fn a_use_case_goes_from_draft_to_approved_versions() {
    let client = test_client(repo()).await;

    let (_, next) = call(&client, VIEWER, "GET", &format!("{BASE}/next-code"), None).await;
    assert_eq!(next["reference_code"], "UC-001");

    let id = export_use_case(&client).await;
    let (_, draft) = call(&client, AUTHOR, "GET", &format!("{BASE}/{id}/draft"), None).await;
    assert_eq!(draft["labels"]["s_Pick01"], "2");
    assert_eq!(draft["labels"]["b_None01"], "A2");
    assert_eq!(draft["labels"]["s_Ask001"], "A2.1");
    assert_eq!(draft["updated_by"], AUTHOR);

    // Moving the origin relabels the branch and says so.
    let (status, moved) = ops(
        &client,
        id,
        json!(2),
        json!([{ "op": "move_element", "id": "s_Pick01", "after": null }]),
    )
    .await;
    assert_eq!(status, Status::Ok, "{moved}");
    assert_eq!(
        moved["effects"],
        json!([{ "type": "relabelled", "id": "b_None01", "from": "A2", "to": "A1" }])
    );

    // A write based on an old revision is refused with the current one.
    let (status, stale) = ops(
        &client,
        id,
        json!(2),
        json!([{ "op": "update_text", "id": "s_Make01", "text": "x" }]),
    )
    .await;
    assert_eq!(status, Status::Conflict);
    assert_eq!(stale["code"], "stale_revision");
    assert_eq!(stale["current"]["revision"], 3);

    // Publish version 1.
    let (status, published) = call(
        &client,
        AUTHOR,
        "POST",
        &format!("{BASE}/{id}/draft/publish"),
        Some(json!({ "base_revision": 3, "change_note": "First cut" })),
    )
    .await;
    assert_eq!(status, Status::Created, "{published}");
    let v1 = published["version"]["id"].as_i64().unwrap();
    assert_eq!(published["version"]["version_number"], 1);
    assert_eq!(published["version"]["approval_state"], "draft");
    let (status, _) = call(&client, AUTHOR, "GET", &format!("{BASE}/{id}/draft"), None).await;
    assert_eq!(status, Status::NotFound, "publishing consumes the draft");

    let (_, list) = call(&client, VIEWER, "GET", BASE, None).await;
    assert_eq!(
        list,
        json!([{
            "id": id, "reference_code": "UC-001", "title": "Export requirements",
            "primary_actor": "Engineer", "step_count": 4, "alternative_count": 1,
            "exception_count": 0, "approval_state": "draft", "current_version_number": 1,
            "has_draft": false, "updated_at": list[0]["updated_at"]
        }])
    );

    // The next edit opens a draft from version 1.
    let (status, reopened) = ops(
        &client,
        id,
        Value::Null,
        json!([{ "op": "update_text", "id": "s_Make01", "text": "System generates the file in the chosen format." }]),
    )
    .await;
    assert_eq!(status, Status::Ok, "{reopened}");
    assert_eq!(reopened["revision"], 1);
    assert_eq!(reopened["base_version_id"], v1);

    // Removing a branch origin needs a decision.
    let (status, refused) = ops(
        &client,
        id,
        json!(1),
        json!([{ "op": "remove_element", "id": "s_Pick01" }]),
    )
    .await;
    assert_eq!(status, Status::UnprocessableEntity);
    assert_eq!(refused["code"], "element_has_dependents");
    assert_eq!(refused["element_label"], "1");
    assert_eq!(
        refused["branches"],
        json!([{ "id": "b_None01", "label": "A1", "title": "Nothing selected" }])
    );
    let (status, _) = ops(
        &client,
        id,
        json!(1),
        json!([{ "op": "remove_element", "id": "s_Pick01", "dependents": { "rebranch_to": "s_Open01" } }]),
    )
    .await;
    assert_eq!(status, Status::Ok);
    let (status, v2) = call(
        &client,
        AUTHOR,
        "POST",
        &format!("{BASE}/{id}/draft/publish"),
        Some(json!({ "base_revision": 2 })),
    )
    .await;
    assert_eq!(status, Status::Created);
    let v2_id = v2["version"]["id"].as_i64().unwrap();

    let (_, versions) = call(
        &client,
        VIEWER,
        "GET",
        &format!("{BASE}/{id}/versions"),
        None,
    )
    .await;
    let numbers: Vec<i64> = versions
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["version_number"].as_i64().unwrap())
        .collect();
    assert_eq!(numbers, vec![2, 1]);
    let (_, old) = call(
        &client,
        VIEWER,
        "GET",
        &format!("{BASE}/{id}?version_id={v1}"),
        None,
    )
    .await;
    assert_eq!(old["current_version"]["labels"]["b_None01"], "A1");
    assert!(old["draft"].is_null());

    // Approval: reviewers only, one step at a time.
    let approval = format!("{BASE}/{id}/versions/{v2_id}/approval");
    let (status, _) = call(
        &client,
        AUTHOR,
        "PUT",
        &approval,
        Some(json!({ "state": "reviewed" })),
    )
    .await;
    assert_eq!(status, Status::Forbidden);
    let (status, _) = call(
        &client,
        ADMIN,
        "PUT",
        &approval,
        Some(json!({ "state": "approved" })),
    )
    .await;
    assert_eq!(status, Status::BadRequest);
    let (status, reviewed) = call(
        &client,
        ADMIN,
        "PUT",
        &approval,
        Some(json!({ "state": "reviewed" })),
    )
    .await;
    assert_eq!(
        (status, reviewed["approval_state"].as_str()),
        (Status::Ok, Some("reviewed"))
    );
    let (_, approved) = call(
        &client,
        ADMIN,
        "PUT",
        &approval,
        Some(json!({ "state": "approved" })),
    )
    .await;
    assert_eq!(approved["approval_state"], "approved");
    assert_eq!(approved["approved_by"], ADMIN);

    // Every step is in the audit log.
    let state = client
        .rocket()
        .state::<crate::api_project_bundle_test::test_support::TestAppState>()
        .unwrap();
    let repo = state.repo.read().unwrap();
    let actions: Vec<&str> = repo
        .inner_repo()
        .logs
        .iter()
        .filter(|l| l.entity_type == "USE_CASE")
        .map(|l| l.action_type.as_str())
        .collect();
    assert_eq!(
        actions,
        [
            "CREATE",
            "UPDATE",
            "UPDATE",
            "STATUS_CHANGE",
            "STATUS_CHANGE"
        ]
    );
}

#[rocket::async_test]
async fn members_read_editors_write_and_outsiders_see_nothing() {
    let client = test_client(repo()).await;
    let id = export_use_case(&client).await;

    let (status, _) = call(&client, VIEWER, "GET", &format!("{BASE}/{id}"), None).await;
    assert_eq!(status, Status::Ok);
    let (status, _) = call(
        &client,
        VIEWER,
        "POST",
        BASE,
        Some(json!({ "title": "Mine" })),
    )
    .await;
    assert_eq!(status, Status::Forbidden);
    let (status, _) = call(&client, OUTSIDER, "GET", BASE, None).await;
    assert_eq!(status, Status::Forbidden);

    // A use case of another project is not reachable through this one.
    let (status, _) = call(
        &client,
        AUTHOR,
        "GET",
        &format!("/api/projects/2/use-cases/{id}"),
        None,
    )
    .await;
    assert_eq!(status, Status::Forbidden);
}

#[rocket::async_test]
async fn documents_codes_and_drafts_are_checked() {
    let client = test_client(repo()).await;
    let id = export_use_case(&client).await;

    let (status, body) = call(
        &client,
        AUTHOR,
        "POST",
        BASE,
        Some(json!({ "reference_code": "UC-001", "title": "Again" })),
    )
    .await;
    assert_eq!(status, Status::Conflict, "{body}");
    let (status, _) = call(
        &client,
        AUTHOR,
        "POST",
        BASE,
        Some(json!({ "reference_code": "uc 7", "title": "Again" })),
    )
    .await;
    assert_eq!(status, Status::BadRequest);
    let (status, body) = call(
        &client,
        AUTHOR,
        "POST",
        BASE,
        Some(json!({ "title": "ab" })),
    )
    .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (Status::UnprocessableEntity, Some("invalid_document"))
    );

    // Unknown keys and broken references in a whole-document write.
    let (_, draft) = call(&client, AUTHOR, "GET", &format!("{BASE}/{id}/draft"), None).await;
    let mut doc = draft["document"].clone();
    doc["colour"] = json!("red");
    let (status, body) = call(
        &client,
        AUTHOR,
        "PUT",
        &format!("{BASE}/{id}/draft"),
        Some(json!({ "base_revision": 2, "document": doc })),
    )
    .await;
    assert_eq!(
        (status, body["code"].as_str()),
        (Status::UnprocessableEntity, Some("invalid_document"))
    );
    let mut doc = draft["document"].clone();
    doc["branches"][0]["from_step"] = json!("s_Ask001");
    let (status, body) = call(
        &client,
        AUTHOR,
        "PUT",
        &format!("{BASE}/{id}/draft"),
        Some(json!({ "base_revision": 2, "document": doc })),
    )
    .await;
    assert_eq!(status, Status::UnprocessableEntity);
    assert_eq!(body["errors"][0]["path"], "branches[0].from_step");

    // An empty main scenario cannot be published; a never-published draft
    // cannot be discarded.
    let (status, empty) = call(
        &client,
        AUTHOR,
        "POST",
        BASE,
        Some(json!({ "title": "Empty one" })),
    )
    .await;
    assert_eq!(status, Status::Created);
    let empty_id = empty["id"].as_i64().unwrap();
    let (status, body) = call(
        &client,
        AUTHOR,
        "POST",
        &format!("{BASE}/{empty_id}/draft/publish"),
        Some(json!({ "base_revision": 1 })),
    )
    .await;
    assert_eq!(
        (status, body["errors"][0]["path"].as_str()),
        (Status::UnprocessableEntity, Some("main"))
    );
    let (status, _) = call(
        &client,
        AUTHOR,
        "DELETE",
        &format!("{BASE}/{empty_id}/draft"),
        None,
    )
    .await;
    assert_eq!(status, Status::Conflict);

    // An id dropped by a published version never comes back.
    let publish = format!("{BASE}/{id}/draft/publish");
    let (status, _) = call(
        &client,
        AUTHOR,
        "POST",
        &publish,
        Some(json!({ "base_revision": 2 })),
    )
    .await;
    assert_eq!(status, Status::Created);
    let (status, _) = ops(
        &client,
        id,
        Value::Null,
        json!([{ "op": "remove_branch", "id": "b_None01" }]),
    )
    .await;
    assert_eq!(status, Status::Ok);
    let (status, _) = call(
        &client,
        AUTHOR,
        "POST",
        &publish,
        Some(json!({ "base_revision": 1 })),
    )
    .await;
    assert_eq!(status, Status::Created);
    let (status, body) = ops(&client, id, Value::Null, json!([
        { "op": "add_element", "list": "main", "element": { "id": "s_Ask001", "text": "Back again." } }
    ])).await;
    assert_eq!(status, Status::UnprocessableEntity, "{body}");
    assert_eq!(body["errors"][0]["path"], "s_Ask001");
    // Undoing a removal within the same draft is fine: the id is in its base.
    let (status, body) = ops(
        &client,
        id,
        Value::Null,
        json!([{ "op": "remove_element", "id": "s_Make01" }]),
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    let (status, body) = ops(&client, id, json!(1), json!([
        { "op": "add_element", "list": "main", "element": { "id": "s_Make01", "text": "System generates the file." } }
    ])).await;
    assert_eq!(status, Status::Ok, "{body}");

    // Discarding a draft of a published use case, then deleting it.
    let (status, _) = call(
        &client,
        AUTHOR,
        "DELETE",
        &format!("{BASE}/{id}/draft"),
        None,
    )
    .await;
    assert_eq!(status, Status::NoContent);
    let (status, _) = call(&client, AUTHOR, "DELETE", &format!("{BASE}/{id}"), None).await;
    assert_eq!(status, Status::NoContent);
    let (status, _) = call(&client, AUTHOR, "GET", &format!("{BASE}/{id}"), None).await;
    assert_eq!(status, Status::NotFound);
}

#[rocket::async_test]
async fn an_archived_project_is_read_only() {
    let mut repo = repo();
    repo.projects.get_mut(&1).unwrap().archived_at = Some(timestamp());
    let client = test_client(repo).await;
    let (status, body) = call(
        &client,
        AUTHOR,
        "POST",
        BASE,
        Some(json!({ "title": "Export requirements" })),
    )
    .await;
    assert_eq!(status, Status::Forbidden, "{body}");
    let (status, _) = call(&client, AUTHOR, "GET", BASE, None).await;
    assert_eq!(status, Status::Ok);
}
