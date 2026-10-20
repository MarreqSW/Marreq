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
            "has_draft": false, "link_count": 0, "suspect_count": 0,
            "updated_at": list[0]["updated_at"]
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

#[rocket::async_test]
async fn links_follow_requirements_and_the_published_elements() {
    let client = test_client(repo()).await;
    let id = export_use_case(&client).await;
    let links = format!("{BASE}/{id}/links");

    // A step link to a requirement and a use case link to a verification.
    let (status, step_link) = call(
        &client,
        AUTHOR,
        "POST",
        &links,
        Some(json!({ "element_id": "s_Pick01", "requirement_id": 1, "rationale": "Selection" })),
    )
    .await;
    assert_eq!(status, Status::Created, "{step_link}");
    assert_eq!(step_link["element_label"], "2");
    assert_eq!(step_link["target_reference_code"], "SAT-PWR-001");
    assert_eq!(step_link["linked_requirement_version_id"], 10);
    let (status, _) = call(
        &client,
        AUTHOR,
        "POST",
        &links,
        Some(json!({ "verification_id": 1 })),
    )
    .await;
    assert_eq!(status, Status::Created);

    let (status, _) = call(
        &client,
        AUTHOR,
        "POST",
        &links,
        Some(json!({ "element_id": "s_Pick01", "requirement_id": 1 })),
    )
    .await;
    assert_eq!(status, Status::Conflict, "the same link twice");
    let (status, _) = call(
        &client,
        AUTHOR,
        "POST",
        &links,
        Some(json!({ "element_id": "s_Nope01", "requirement_id": 1 })),
    )
    .await;
    assert_eq!(status, Status::BadRequest);
    let (status, _) = call(
        &client,
        AUTHOR,
        "POST",
        &links,
        Some(json!({ "requirement_id": 1, "verification_id": 1 })),
    )
    .await;
    assert_eq!(status, Status::BadRequest);
    let (status, _) = call(
        &client,
        AUTHOR,
        "POST",
        &links,
        Some(json!({ "requirement_id": 999 })),
    )
    .await;
    assert_eq!(status, Status::NotFound);
    let (status, _) = call(
        &client,
        VIEWER,
        "POST",
        &links,
        Some(json!({ "requirement_id": 1 })),
    )
    .await;
    assert_eq!(status, Status::Forbidden);

    // Reverse lookups.
    let (_, used) = call(
        &client,
        VIEWER,
        "GET",
        "/api/projects/1/requirements/1/use-cases",
        None,
    )
    .await;
    assert_eq!(used[0]["reference_code"], "UC-001");
    assert_eq!(used[0]["element_label"], "2");
    let (_, used) = call(
        &client,
        VIEWER,
        "GET",
        "/api/projects/1/verifications/1/use-cases",
        None,
    )
    .await;
    assert_eq!(
        (
            used[0]["element_id"].clone(),
            used.as_array().unwrap().len()
        ),
        (Value::Null, 1)
    );

    // A requirement change makes the link suspect, as it does matrix rows.
    let (status, _) = call(
        &client,
        ADMIN,
        "PUT",
        "/api/projects/1/requirements/1/versions/10/approval",
        Some(json!({ "state": "reviewed" })),
    )
    .await;
    assert_eq!(status, Status::Ok);
    let (_, rows) = call(
        &client,
        VIEWER,
        "GET",
        &format!("{BASE}?has_suspect=true"),
        None,
    )
    .await;
    assert_eq!(
        (
            rows[0]["link_count"].as_i64(),
            rows[0]["suspect_count"].as_i64()
        ),
        (Some(2), Some(1))
    );
    let (_, listed) = call(&client, VIEWER, "GET", &links, None).await;
    let suspect = listed
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["suspect"] == true)
        .unwrap()
        .clone();
    assert_eq!(suspect["suspect_reason"], "Approval state changed");
    let clear = format!("{links}/{}/clear_suspect", suspect["id"]);
    let (_, cleared) = call(&client, AUTHOR, "POST", &clear, None).await;
    assert_eq!(cleared, json!({ "status": "ok", "cleared": true }));
    let (_, again) = call(&client, AUTHOR, "POST", &clear, None).await;
    assert_eq!(again["status"], "no_change");
    let (_, rows) = call(
        &client,
        VIEWER,
        "GET",
        &format!("{BASE}?requirement_id=1"),
        None,
    )
    .await;
    assert_eq!(
        (
            rows.as_array().unwrap().len(),
            rows[0]["suspect_count"].as_i64()
        ),
        (1, Some(0))
    );
    let (_, rows) = call(
        &client,
        VIEWER,
        "GET",
        &format!("{BASE}?requirement_id=2"),
        None,
    )
    .await;
    assert_eq!(rows, json!([]));

    // Publishing a version without the step removes its link and says so.
    let (status, _) = ops(
        &client,
        id,
        json!(2),
        json!([{ "op": "remove_element", "id": "s_Pick01", "dependents": "cascade" }]),
    )
    .await;
    assert_eq!(status, Status::Ok);
    let (status, published) = call(
        &client,
        AUTHOR,
        "POST",
        &format!("{BASE}/{id}/draft/publish"),
        Some(json!({ "base_revision": 3 })),
    )
    .await;
    assert_eq!(status, Status::Created);
    assert_eq!(
        published["removed_links"][0]["target_reference_code"],
        "SAT-PWR-001"
    );
    let (_, listed) = call(&client, VIEWER, "GET", &links, None).await;
    assert_eq!(
        listed.as_array().unwrap().len(),
        1,
        "the use case link stays"
    );

    // Deleting a link.
    let link_id = listed[0]["id"].as_i64().unwrap();
    let (status, _) = call(
        &client,
        AUTHOR,
        "DELETE",
        &format!("{links}/{link_id}"),
        None,
    )
    .await;
    assert_eq!(status, Status::NoContent);
    let (_, listed) = call(&client, VIEWER, "GET", &links, None).await;
    assert_eq!(listed, json!([]));
}

#[rocket::async_test]
async fn diffs_compare_the_draft_and_versions_structurally() {
    let client = test_client(repo()).await;
    let id = export_use_case(&client).await;

    // Before the first publish everything is added.
    let (_, first) = call(
        &client,
        VIEWER,
        "GET",
        &format!("{BASE}/{id}/draft/diff"),
        None,
    )
    .await;
    assert_eq!(first["from"], Value::Null);
    assert_eq!(first["summary"]["added"], 5);
    let publish = format!("{BASE}/{id}/draft/publish");
    let (_, v1) = call(
        &client,
        AUTHOR,
        "POST",
        &publish,
        Some(json!({ "base_revision": 2 })),
    )
    .await;
    let v1 = v1["version"]["id"].as_i64().unwrap();

    let (status, _) = ops(
        &client,
        id,
        Value::Null,
        json!([
            { "op": "move_element", "id": "s_Make01", "after": null },
            { "op": "update_text", "id": "s_Open01", "text": "User opens the export page." },
            { "op": "update_branch", "id": "b_None01", "from_step": "s_Open01" }
        ]),
    )
    .await;
    assert_eq!(status, Status::Ok);
    let (status, diff) = call(
        &client,
        VIEWER,
        "GET",
        &format!("{BASE}/{id}/draft/diff"),
        None,
    )
    .await;
    assert_eq!(status, Status::Ok, "{diff}");
    assert_eq!(diff["from"]["version_number"], 1);
    assert_eq!(diff["to"]["draft_revision"], 1);
    assert_eq!(
        diff["summary"],
        json!({ "added": 0, "removed": 0, "edited": 1, "moved": 1, "rebranched": 1 })
    );
    let edited = diff["elements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == "s_Open01")
        .unwrap();
    assert_eq!(
        edited["text_diff"][1],
        json!({ "op": "delete", "text": "requirements" })
    );

    let (_, v2) = call(
        &client,
        AUTHOR,
        "POST",
        &publish,
        Some(json!({ "base_revision": 1 })),
    )
    .await;
    let v2 = v2["version"]["id"].as_i64().unwrap();
    let (_, versions) = call(
        &client,
        VIEWER,
        "GET",
        &format!("{BASE}/{id}/versions/{v1}/diff/{v2}"),
        None,
    )
    .await;
    assert_eq!(versions["summary"], diff["summary"]);
    let (_, all) = call(
        &client,
        VIEWER,
        "GET",
        &format!("{BASE}/{id}/versions/{v1}/diff/{v2}?include_unchanged=true"),
        None,
    )
    .await;
    assert_eq!(all["elements"].as_array().unwrap().len(), 5);
    let (status, _) = call(
        &client,
        VIEWER,
        "GET",
        &format!("{BASE}/{id}/draft/diff"),
        None,
    )
    .await;
    assert_eq!(status, Status::NotFound);
}

fn notifications(client: &Client) -> Vec<(i32, String, Option<String>, Option<i32>)> {
    let state = client
        .rocket()
        .state::<crate::api_project_bundle_test::test_support::TestAppState>()
        .unwrap();
    let repo = state.repo.read().unwrap();
    repo.inner_repo()
        .notifications
        .iter()
        .map(|n| {
            (
                n.user_id,
                n.notification_type.clone(),
                n.entity_type.clone(),
                n.entity_id,
            )
        })
        .collect()
}

#[rocket::async_test]
async fn comments_anchor_to_steps_and_notify() {
    let mut repo = repo();
    // The viewer reviews: publishing asks them, and approvals reach the start screen.
    repo.project_reviewers.insert(1, vec![VIEWER]);
    let client = test_client(repo).await;
    let id = export_use_case(&client).await;
    let (_, published) = call(
        &client,
        AUTHOR,
        "POST",
        &format!("{BASE}/{id}/draft/publish"),
        Some(json!({ "base_revision": 2 })),
    )
    .await;
    let v1 = published["version"]["id"].as_i64().unwrap();
    assert_eq!(
        notifications(&client),
        vec![(
            VIEWER,
            "approval_requested".into(),
            Some("use_case".into()),
            Some(id as i32)
        )]
    );

    let comments = format!("{BASE}/{id}/comments");
    let (status, comment) = call(&client, ADMIN, "POST", &comments, Some(json!({
        "body": "@viewer is step 2 right? cc @outsider", "use_case_version_id": v1, "element_id": "s_Pick01"
    }))).await;
    assert_eq!(status, Status::Created, "{comment}");
    assert_eq!(comment["element_label"], "2");
    assert_eq!(comment["author_name"], "name");
    let notes = notifications(&client);
    assert_eq!(
        notes[1..]
            .iter()
            .map(|(u, t, _, _)| (*u, t.as_str()))
            .collect::<Vec<_>>(),
        vec![(VIEWER, "mentioned"), (AUTHOR, "comment_added")],
        "the outsider is not a member; the author created the use case"
    );

    let (status, _) = call(
        &client,
        AUTHOR,
        "POST",
        &comments,
        Some(json!({ "body": "On the use case." })),
    )
    .await;
    assert_eq!(status, Status::Created);
    let (status, _) = call(
        &client,
        AUTHOR,
        "POST",
        &comments,
        Some(json!({ "body": "x", "element_id": "s_Nope01" })),
    )
    .await;
    assert_eq!(status, Status::BadRequest);
    let (status, _) = call(
        &client,
        AUTHOR,
        "POST",
        &comments,
        Some(json!({ "body": "  " })),
    )
    .await;
    assert_eq!(status, Status::BadRequest);
    let (status, _) = call(
        &client,
        VIEWER,
        "POST",
        &comments,
        Some(json!({ "body": "Viewers read only." })),
    )
    .await;
    assert_eq!(status, Status::Forbidden);

    let (_, all) = call(&client, VIEWER, "GET", &comments, None).await;
    assert_eq!(all.as_array().unwrap().len(), 2);
    let (_, on_step) = call(
        &client,
        VIEWER,
        "GET",
        &format!("{comments}?element_id=s_Pick01"),
        None,
    )
    .await;
    assert_eq!(on_step.as_array().unwrap().len(), 1);
    assert_eq!(on_step[0]["use_case_version_id"], v1);

    // The viewer, a reviewer, sees the mention and, once reviewed, the approval.
    let (_, attention) = call(&client, VIEWER, "GET", "/api/home/attention", None).await;
    let mention = attention["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["notification_type"] == "mentioned")
        .unwrap();
    assert_eq!(mention["use_case_id"], id);
    let (status, _) = call(
        &client,
        VIEWER,
        "PUT",
        &format!("{BASE}/{id}/versions/{v1}/approval"),
        Some(json!({ "state": "reviewed" })),
    )
    .await;
    assert_eq!(status, Status::Ok);
    let (_, attention) = call(&client, VIEWER, "GET", "/api/home/attention", None).await;
    let approval = attention["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["kind"] == "approval" && i["use_case_id"] == id)
        .unwrap();
    assert_eq!(approval["title"], "UC-001 v1 is waiting for your approval");
}

#[rocket::async_test]
async fn baselines_record_use_case_versions_and_links() {
    let client = test_client(repo()).await;
    let id = export_use_case(&client).await;
    let publish = format!("{BASE}/{id}/draft/publish");
    let (_, v1) = call(
        &client,
        AUTHOR,
        "POST",
        &publish,
        Some(json!({ "base_revision": 2 })),
    )
    .await;
    let v1 = v1["version"]["id"].as_i64().unwrap();
    call(
        &client,
        AUTHOR,
        "POST",
        &format!("{BASE}/{id}/links"),
        Some(json!({ "element_id": "s_Pick01", "requirement_id": 1 })),
    )
    .await;
    // An unpublished use case is not recorded.
    call(
        &client,
        AUTHOR,
        "POST",
        BASE,
        Some(json!({ "title": "Not yet published" })),
    )
    .await;

    let (status, a) = call(
        &client,
        ADMIN,
        "POST",
        "/api/projects/1/baselines",
        Some(json!({ "name": "PDR" })),
    )
    .await;
    assert_eq!(status, Status::Ok, "{a}");
    let a = a["id"].as_i64().unwrap();
    let (_, recorded) = call(
        &client,
        VIEWER,
        "GET",
        &format!("/api/projects/1/baselines/{a}/use-cases"),
        None,
    )
    .await;
    assert_eq!(recorded.as_array().unwrap().len(), 1);
    assert_eq!(
        (
            recorded[0]["version_id"].as_i64(),
            recorded[0]["link_count"].as_i64()
        ),
        (Some(v1), Some(1))
    );

    // A newer version: the baseline still points at v1, the diff shows the change.
    ops(&client, id, Value::Null, json!([{ "op": "update_text", "id": "s_Make01", "text": "System generates the export file." }])).await;
    let (_, v2) = call(
        &client,
        AUTHOR,
        "POST",
        &publish,
        Some(json!({ "base_revision": 1 })),
    )
    .await;
    let v2 = v2["version"]["id"].as_i64().unwrap();
    let (_, recorded) = call(
        &client,
        VIEWER,
        "GET",
        &format!("/api/projects/1/baselines/{a}/use-cases"),
        None,
    )
    .await;
    assert_eq!(
        (
            recorded[0]["version_number"].as_i64(),
            recorded[0]["current_version_number"].as_i64()
        ),
        (Some(1), Some(2))
    );
    let (_, diff) = call(
        &client,
        VIEWER,
        "GET",
        &format!("/api/projects/1/baselines/{a}/use-cases/{id}/diff/current"),
        None,
    )
    .await;
    assert_eq!(
        (
            diff["from"]["version_id"].as_i64(),
            diff["to"]["version_id"].as_i64()
        ),
        (Some(v1), Some(v2))
    );
    assert_eq!(diff["summary"]["edited"], 1);

    let (_, b) = call(
        &client,
        ADMIN,
        "POST",
        "/api/projects/1/baselines",
        Some(json!({ "name": "CDR" })),
    )
    .await;
    let b = b["id"].as_i64().unwrap();
    let (_, compared) = call(
        &client,
        VIEWER,
        "GET",
        &format!("/api/projects/1/baselines/diff?baseline_a={a}&baseline_b={b}"),
        None,
    )
    .await;
    assert_eq!(
        compared["use_cases_changed"],
        json!([{ "use_case_id": id, "version_a": v1, "version_b": v2 }])
    );
    assert_eq!(compared["use_cases_only_in_a"], json!([]));

    // A baselined use case cannot be deleted.
    let (status, body) = call(&client, AUTHOR, "DELETE", &format!("{BASE}/{id}"), None).await;
    assert_eq!(status, Status::Conflict, "{body}");
}

#[rocket::async_test]
async fn the_global_search_finds_use_cases() {
    let client = test_client(repo()).await;
    let id = export_use_case(&client).await;
    for q in ["UC-001", "export req", "selects requirements", "engineer"] {
        let (status, hits) = call(
            &client,
            VIEWER,
            "GET",
            &format!("/api/search?q={}", q.replace(' ', "%20")),
            None,
        )
        .await;
        assert_eq!(status, Status::Ok);
        let hit = hits
            .as_array()
            .unwrap()
            .iter()
            .find(|h| h["kind"] == "use_case")
            .unwrap_or_else(|| panic!("{q}: {hits}"));
        assert_eq!(
            (hit["use_case_id"].as_i64(), hit["requirement_id"].clone()),
            (Some(id), Value::Null),
            "{q}"
        );
        assert_eq!(hit["title"], "Export requirements");
    }
    let (_, hits) = call(&client, VIEWER, "GET", "/api/search?q=SAT-PWR", None).await;
    assert_eq!(hits[0]["kind"], "requirement");
    let (_, hits) = call(&client, OUTSIDER, "GET", "/api/search?q=UC-001", None).await;
    assert_eq!(hits, json!([]));
}

#[rocket::async_test]
async fn the_project_bundle_carries_use_cases_and_their_links() {
    use crate::api_project_bundle_test::test_support::json_file_body;
    let client = test_client(repo()).await;
    let id = export_use_case(&client).await;
    call(
        &client,
        AUTHOR,
        "POST",
        &format!("{BASE}/{id}/draft/publish"),
        Some(json!({ "base_revision": 2 })),
    )
    .await;
    call(
        &client,
        AUTHOR,
        "POST",
        &format!("{BASE}/{id}/links"),
        Some(json!({ "element_id": "s_Pick01", "requirement_id": 1, "rationale": "Selection" })),
    )
    .await;
    call(
        &client,
        AUTHOR,
        "POST",
        &format!("{BASE}/{id}/links"),
        Some(json!({ "verification_id": 1 })),
    )
    .await;
    call(
        &client,
        AUTHOR,
        "POST",
        BASE,
        Some(json!({ "reference_code": "UC-002", "title": "Only a draft" })),
    )
    .await;

    let exported = client
        .get("/api/projects/1/exports/bundle.json")
        .private_cookie(session_cookie(&client, ADMIN))
        .dispatch()
        .await;
    let json = exported.into_string().await.unwrap();
    let bundle: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(bundle["use_cases"].as_array().unwrap().len(), 2);
    assert_eq!(
        bundle["use_cases"][0]["links"][0]["requirement_reference_code"],
        "SAT-PWR-001"
    );

    let (ct, body) = json_file_body("bundle.json", &json);
    let imported = client
        .post("/api/projects/imports/bundle")
        .header(ct)
        .private_cookie(session_cookie(&client, ADMIN))
        .body(body)
        .dispatch()
        .await;
    let result: Value = imported.into_json().await.unwrap();
    assert_eq!(
        (
            result["imported_counts"]["use_cases"].as_i64(),
            result["imported_counts"]["use_case_links"].as_i64()
        ),
        (Some(2), Some(2)),
        "{result}"
    );
    let new_id = result["project_id"].as_i64().unwrap();
    let (_, rows) = call(
        &client,
        ADMIN,
        "GET",
        &format!("/api/projects/{new_id}/use-cases"),
        None,
    )
    .await;
    assert_eq!(
        rows.as_array()
            .unwrap()
            .iter()
            .map(|r| (
                r["reference_code"].as_str().unwrap(),
                r["current_version_number"].as_i64(),
                r["link_count"].as_i64()
            ))
            .collect::<Vec<_>>(),
        vec![("UC-001", Some(1), Some(2)), ("UC-002", None, Some(0))]
    );
}

async fn call_with_key(client: &Client, path: &str, key: &str, body: Value) -> (Status, Value) {
    let response = client
        .post(path.to_string())
        .private_cookie(session_cookie(client, AUTHOR))
        .header(ContentType::JSON)
        .header(rocket::http::Header::new(
            "Idempotency-Key",
            key.to_string(),
        ))
        .body(body.to_string())
        .dispatch()
        .await;
    let status = response.status();
    (status, response.into_json().await.unwrap_or(Value::Null))
}

#[rocket::async_test]
async fn create_and_publish_replay_with_an_idempotency_key() {
    let client = test_client(repo()).await;
    let body = json!({ "title": "Retried creation" });
    let (s1, first) = call_with_key(&client, BASE, "k-create", body.clone()).await;
    let (s2, again) = call_with_key(&client, BASE, "k-create", body).await;
    assert_eq!((s1, s2), (Status::Created, Status::Created));
    assert_eq!(first["id"], again["id"]);
    let (_, list) = call(&client, AUTHOR, "GET", BASE, None).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    let (status, _) = call_with_key(&client, BASE, "k-create", json!({ "title": "Another" })).await;
    assert_eq!(status, Status::Conflict, "same key, different payload");

    let id = first["id"].as_i64().unwrap();
    ops(
        &client,
        id,
        json!(1),
        json!([{ "op": "add_element", "list": "main", "element": { "text": "A step." } }]),
    )
    .await;
    let publish = format!("{BASE}/{id}/draft/publish");
    let (s1, v1) = call_with_key(
        &client,
        &publish,
        "k-publish",
        json!({ "base_revision": 2 }),
    )
    .await;
    let (s2, replay) = call_with_key(
        &client,
        &publish,
        "k-publish",
        json!({ "base_revision": 2 }),
    )
    .await;
    assert_eq!((s1, s2), (Status::Created, Status::Created), "{replay}");
    assert_eq!(v1["version"]["id"], replay["version"]["id"]);
    let (_, versions) = call(
        &client,
        AUTHOR,
        "GET",
        &format!("{BASE}/{id}/versions"),
        None,
    )
    .await;
    assert_eq!(versions.as_array().unwrap().len(), 1);
}

#[rocket::async_test]
async fn a_comment_replays_with_an_idempotency_key() {
    let client = test_client(repo()).await;
    let (_, created) = call_with_key(&client, BASE, "k-uc", json!({ "title": "Commented" })).await;
    let id = created["id"].as_i64().unwrap();
    let comments = format!("{BASE}/{id}/comments");
    let body = json!({ "body": "Is step 1 enough?" });
    let (s1, first) = call_with_key(&client, &comments, "k-comment", body.clone()).await;
    let (s2, again) = call_with_key(&client, &comments, "k-comment", body).await;
    assert_eq!((s1, s2), (Status::Created, Status::Created), "{again}");
    assert_eq!(first["id"], again["id"]);
    let (_, list) = call(&client, AUTHOR, "GET", &comments, None).await;
    assert_eq!(
        list.as_array().unwrap().len(),
        1,
        "the retry posted no second comment"
    );
    let (status, _) =
        call_with_key(&client, &comments, "k-comment", json!({ "body": "Other" })).await;
    assert_eq!(status, Status::Conflict, "same key, different payload");
    // Without a key every post is a new comment.
    let (_, _) = call(
        &client,
        AUTHOR,
        "POST",
        &comments,
        Some(json!({ "body": "Again" })),
    )
    .await;
    let (_, list) = call(&client, AUTHOR, "GET", &comments, None).await;
    assert_eq!(list.as_array().unwrap().len(), 2);
}
