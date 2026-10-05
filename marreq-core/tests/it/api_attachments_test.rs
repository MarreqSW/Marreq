// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Attachments and project storage quotas (issue #241) through the full API router.

#![cfg(feature = "test-helpers")]

use chrono::{NaiveDate, NaiveDateTime};
use marreq_core::app::AppState;
use marreq_core::auth::session::test_session_cookie_for;
use marreq_core::models::*;
use marreq_core::permissions::{ROLE_ADMIN, ROLE_VIEWER};
use marreq_core::repository::{CacheRepository, diesel_repo_mock::DieselRepoMock};
use marreq_core::status_enums::ProjectStatus;
use marreq_core::storage::{AttachmentStorage, AttachmentsConfig};
use rocket::http::{ContentType, Cookie, Status};
use rocket::local::asynchronous::{Client, LocalResponse};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

type TestAppState = AppState<CacheRepository<DieselRepoMock>>;

const LEAD: i32 = 1; // project Admin role, not an instance admin
const VIEWER: i32 = 2;
const SITE_ADMIN: i32 = 3;
const REQ: i32 = 10;
const OTHER_PROJECT_REQ: i32 = 20;
const VER: i32 = 30;

const MAX_FILE: u64 = 4096;
const QUOTA: u64 = 10_000;

const PDF_HEAD: &[u8] = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n";

fn pdf(size: usize, fill: u8) -> Vec<u8> {
    let mut bytes = PDF_HEAD.to_vec();
    bytes.resize(size, fill);
    bytes
}

fn timestamp() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

fn project(id: i32, slug: &str) -> Project {
    Project {
        id,
        name: slug.into(),
        description: None,
        creation_date: None,
        update_date: None,
        status: ProjectStatus::Active,
        owner_id: Some(LEAD),
        slug: slug.into(),
        group_id: None,
        archived_at: None,
        archived_by: None,
    }
}

fn requirement(id: i32, project_id: i32) -> Requirement {
    Requirement {
        id,
        current_version_id: None,
        same_as_current: None,
        title: format!("Requirement {id}"),
        description: String::new(),
        status_id: 1,
        author_id: LEAD,
        reviewer_id: LEAD,
        reference_code: format!("REQ-{id}"),
        category_id: 1,
        parent_id: None,
        creation_date: timestamp(),
        update_date: timestamp(),
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

fn repo() -> DieselRepoMock {
    let mut repo = DieselRepoMock::default();
    let mut lead = DieselRepoMock::make_user(LEAD, "lead", "password");
    lead.name = "Project Lead".into();
    repo.users.insert(LEAD, lead);
    repo.users.insert(
        VIEWER,
        DieselRepoMock::make_user(VIEWER, "viewer", "password"),
    );
    let mut admin = DieselRepoMock::make_user(SITE_ADMIN, "root", "password");
    admin.is_admin = true;
    repo.users.insert(SITE_ADMIN, admin);
    repo.projects.insert(3, project(3, "lunar-lander"));
    repo.projects.insert(4, project(4, "other"));
    for (user_id, role) in [(LEAD, ROLE_ADMIN), (VIEWER, ROLE_VIEWER)] {
        repo.project_members.push(ProjectMember {
            project_id: 3,
            user_id,
            role,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
    }
    repo.requirements.insert(REQ, requirement(REQ, 3));
    repo.requirements
        .insert(OTHER_PROJECT_REQ, requirement(OTHER_PROJECT_REQ, 4));
    repo.verifications.insert(
        VER,
        Verification {
            id: VER,
            name: "Thermal vacuum test".into(),
            reference_code: "VER-1".into(),
            description: String::new(),
            source: String::new(),
            status_id: 1,
            parent_id: None,
            project_id: 3,
            verification_method_id: None,
            author_id: LEAD,
            reviewer_id: LEAD,
            status_set_by: None,
            status_set_at: None,
        },
    );
    repo
}

struct Harness {
    client: Client,
    dir: PathBuf,
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

async fn harness() -> Harness {
    marreq_core::deployment::install_test_server_mode();
    let suffix: u64 = rand::random();
    let dir = std::env::temp_dir().join(format!("marreq-attachments-it-{suffix:016x}"));
    let storage = Arc::new(AttachmentStorage::local(AttachmentsConfig {
        dir: dir.clone(),
        max_file_bytes: MAX_FILE,
        default_project_quota_bytes: QUOTA,
        ..AttachmentsConfig::default()
    }));
    let state: TestAppState = AppState {
        repo: Arc::new(RwLock::new(CacheRepository::new(repo(), 0))),
    };
    let rocket = rocket::build()
        .manage(state)
        .manage(storage)
        .manage(marreq_core::auth::AuthConfig::default())
        .manage(marreq_core::auth::rate_limiter::LoginRateLimiter::new())
        .mount("/api", marreq_core::api::routes());
    Harness {
        client: Client::tracked(rocket).await.expect("rocket instance"),
        dir,
    }
}

fn session(client: &Client, user_id: i32) -> Cookie<'static> {
    test_session_cookie_for(
        client.rocket().state::<TestAppState>().expect("state"),
        user_id,
    )
}

const BOUNDARY: &str = "marreq-test-boundary";

fn multipart(entity_type: &str, entity_id: i32, filename: &str, bytes: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, value) in [
        ("entity_type", entity_type.to_string()),
        ("entity_id", entity_id.to_string()),
    ] {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

async fn upload<'c>(
    h: &'c Harness,
    user_id: Option<i32>,
    entity: (&str, i32),
    filename: &str,
    bytes: &[u8],
) -> LocalResponse<'c> {
    let mut req = h
        .client
        .post("/api/projects/3/attachments")
        .header(ContentType::new("multipart", "form-data").with_params(("boundary", BOUNDARY)))
        .body(multipart(entity.0, entity.1, filename, bytes));
    if let Some(uid) = user_id {
        req = req.private_cookie(session(&h.client, uid));
    }
    req.dispatch().await
}

async fn json_of(res: LocalResponse<'_>) -> Value {
    res.into_json().await.expect("json body")
}

async fn list(h: &Harness, user_id: i32, entity: (&str, i32)) -> Value {
    let res = h
        .client
        .get(format!(
            "/api/projects/3/attachments?entity_type={}&entity_id={}",
            entity.0, entity.1
        ))
        .private_cookie(session(&h.client, user_id))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    json_of(res).await
}

fn blob_count(h: &Harness) -> usize {
    fn walk(dir: &std::path::Path) -> usize {
        std::fs::read_dir(dir)
            .map(|entries| {
                entries
                    .flatten()
                    .map(|e| {
                        let path = e.path();
                        if path.is_dir() {
                            if path.file_name().is_some_and(|n| n == "tmp") {
                                0
                            } else {
                                walk(&path)
                            }
                        } else {
                            1
                        }
                    })
                    .sum()
            })
            .unwrap_or(0)
    }
    walk(&h.dir)
}

#[rocket::async_test]
async fn upload_list_download_and_delete() {
    let h = harness().await;
    let bytes = pdf(1200, b'a');
    let res = upload(
        &h,
        Some(LEAD),
        ("requirement", REQ),
        "Thermal Größe.pdf",
        &bytes,
    )
    .await;
    assert_eq!(res.status(), Status::Created);
    let created = json_of(res).await;
    assert_eq!(created["filename"], "Thermal Größe.pdf");
    assert_eq!(created["content_type"], "application/pdf");
    assert_eq!(created["size_bytes"], 1200);
    assert_eq!(created["uploaded_by_name"], "Project Lead");
    let id = created["id"].as_i64().unwrap();

    let listed = list(&h, VIEWER, ("requirement", REQ)).await;
    assert_eq!(listed.as_array().unwrap().len(), 1);
    assert_eq!(listed[0]["id"], id);

    let res = h
        .client
        .get(format!("/api/projects/3/attachments/{id}/download"))
        .private_cookie(session(&h.client, VIEWER))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(
        res.headers().get_one("Content-Type"),
        Some("application/pdf")
    );
    let disposition = res
        .headers()
        .get_one("Content-Disposition")
        .unwrap()
        .to_string();
    assert!(disposition.starts_with("attachment; filename=\"Thermal Gr__e.pdf\""));
    assert!(disposition.contains("filename*=UTF-8''Thermal%20Gr%C3%B6%C3%9Fe.pdf"));
    assert_eq!(
        res.headers().get_one("X-Content-Type-Options"),
        Some("nosniff")
    );
    assert_eq!(
        res.headers().get_one("Content-Security-Policy"),
        Some("default-src 'none'; sandbox")
    );
    assert_eq!(res.into_bytes().await.unwrap(), bytes);
    assert_eq!(blob_count(&h), 1);

    // Viewers can read but not delete.
    let res = h
        .client
        .delete(format!("/api/projects/3/attachments/{id}"))
        .private_cookie(session(&h.client, VIEWER))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Forbidden);

    let res = h
        .client
        .delete(format!("/api/projects/3/attachments/{id}"))
        .private_cookie(session(&h.client, LEAD))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::NoContent);
    assert!(
        list(&h, LEAD, ("requirement", REQ))
            .await
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(blob_count(&h), 0, "an unreferenced file is removed");
    let res = h
        .client
        .get(format!("/api/projects/3/attachments/{id}/download"))
        .private_cookie(session(&h.client, LEAD))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::NotFound);
}

#[rocket::async_test]
async fn upload_requires_a_session_and_edit_permission() {
    let h = harness().await;
    let bytes = pdf(100, b'a');
    let res = upload(&h, None, ("requirement", REQ), "a.pdf", &bytes).await;
    assert_eq!(res.status(), Status::Unauthorized);
    let res = upload(&h, Some(VIEWER), ("requirement", REQ), "a.pdf", &bytes).await;
    assert_eq!(res.status(), Status::Forbidden);
    assert_eq!(blob_count(&h), 0);
}

#[rocket::async_test]
async fn upload_rejects_entities_of_other_projects_and_bad_entity_types() {
    let h = harness().await;
    let bytes = pdf(100, b'a');
    let res = upload(
        &h,
        Some(LEAD),
        ("requirement", OTHER_PROJECT_REQ),
        "a.pdf",
        &bytes,
    )
    .await;
    assert_eq!(res.status(), Status::NotFound);
    let res = upload(&h, Some(LEAD), ("requirement", 999), "a.pdf", &bytes).await;
    assert_eq!(res.status(), Status::NotFound);
    let res = upload(&h, Some(LEAD), ("project", 3), "a.pdf", &bytes).await;
    assert_eq!(res.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn upload_rejects_disallowed_or_mismatched_types() {
    let h = harness().await;
    let res = upload(
        &h,
        Some(LEAD),
        ("requirement", REQ),
        "page.html",
        b"<html></html>",
    )
    .await;
    assert_eq!(res.status(), Status::UnsupportedMediaType);
    let body = json_of(res).await;
    assert!(body["message"].as_str().unwrap().contains("allowed types"));

    let res = upload(
        &h,
        Some(LEAD),
        ("requirement", REQ),
        "fake.pdf",
        b"not really a pdf",
    )
    .await;
    assert_eq!(res.status(), Status::UnsupportedMediaType);

    let res = upload(
        &h,
        Some(LEAD),
        ("verification", VER),
        "log.csv",
        b"t,temp\n0,21.5\n",
    )
    .await;
    assert_eq!(res.status(), Status::Created);
    assert_eq!(json_of(res).await["content_type"], "text/csv");
    assert_eq!(blob_count(&h), 1, "rejected uploads leave no files");
}

#[rocket::async_test]
async fn upload_enforces_the_per_file_limit_and_the_project_quota() {
    let h = harness().await;
    let res = upload(
        &h,
        Some(LEAD),
        ("requirement", REQ),
        "big.pdf",
        &pdf(MAX_FILE as usize + 1, b'x'),
    )
    .await;
    assert_eq!(res.status(), Status::PayloadTooLarge);
    assert!(
        json_of(res).await["message"]
            .as_str()
            .unwrap()
            .contains("at most")
    );

    // Quota is 10,000 bytes: two 4,000-byte files fit, a third distinct one does not.
    for fill in *b"ab" {
        let res = upload(
            &h,
            Some(LEAD),
            ("requirement", REQ),
            "part.pdf",
            &pdf(4000, fill),
        )
        .await;
        assert_eq!(res.status(), Status::Created);
    }
    let res = upload(
        &h,
        Some(LEAD),
        ("verification", VER),
        "part.pdf",
        &pdf(4000, b'c'),
    )
    .await;
    assert_eq!(res.status(), Status::PayloadTooLarge);
    let message = json_of(res).await["message"].as_str().unwrap().to_string();
    assert!(
        message.contains("Project storage limit reached"),
        "{message}"
    );

    // The same content again is stored once, so it does not count twice.
    let res = upload(
        &h,
        Some(LEAD),
        ("verification", VER),
        "copy.pdf",
        &pdf(4000, b'a'),
    )
    .await;
    assert_eq!(res.status(), Status::Created);
    assert_eq!(blob_count(&h), 2);

    let res = h
        .client
        .get("/api/projects/3/storage")
        .private_cookie(session(&h.client, VIEWER))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let storage = json_of(res).await;
    assert_eq!(storage["used_bytes"], 8000);
    assert_eq!(storage["quota_bytes"], QUOTA);
    assert_eq!(storage["quota_is_default"], true);
    assert_eq!(storage["max_file_bytes"], MAX_FILE);
    assert!(
        storage["allowed_extensions"]
            .as_array()
            .unwrap()
            .contains(&json!("pdf"))
    );
}

#[rocket::async_test]
async fn only_instance_admins_change_the_quota() {
    let h = harness().await;
    let put = |user_id: i32, body: Value| {
        h.client
            .put("/api/projects/3/storage/quota")
            .header(ContentType::JSON)
            .private_cookie(session(&h.client, user_id))
            .body(body.to_string())
            .dispatch()
    };
    assert_eq!(
        put(LEAD, json!({ "quota_mb": 5 })).await.status(),
        Status::Forbidden
    );

    let res = put(SITE_ADMIN, json!({ "quota_mb": 5 })).await;
    assert_eq!(res.status(), Status::Ok);
    let body = json_of(res).await;
    assert_eq!(body["quota_bytes"], 5 * 1024 * 1024);
    assert_eq!(body["quota_is_default"], false);

    // With 5 MB the earlier limit no longer applies.
    for fill in *b"abc" {
        let res = upload(
            &h,
            Some(LEAD),
            ("requirement", REQ),
            "part.pdf",
            &pdf(4000, fill),
        )
        .await;
        assert_eq!(res.status(), Status::Created);
    }

    assert_eq!(
        put(SITE_ADMIN, json!({ "quota_mb": 0 })).await.status(),
        Status::BadRequest
    );
    let res = put(SITE_ADMIN, json!({ "quota_mb": null })).await;
    assert_eq!(res.status(), Status::Ok);
    let body = json_of(res).await;
    assert_eq!(body["quota_bytes"], QUOTA);
    assert_eq!(body["quota_is_default"], true);
    // Lowering below usage is allowed; it only blocks new uploads.
    assert_eq!(body["used_bytes"], 12000);
    let res = upload(
        &h,
        Some(LEAD),
        ("requirement", REQ),
        "more.pdf",
        &pdf(100, b'd'),
    )
    .await;
    assert_eq!(res.status(), Status::PayloadTooLarge);
}

#[rocket::async_test]
async fn baselines_keep_deleted_files_downloadable() {
    let h = harness().await;
    let bytes = pdf(1000, b'v');
    let res = upload(&h, Some(LEAD), ("verification", VER), "report.pdf", &bytes).await;
    assert_eq!(res.status(), Status::Created);
    let id = json_of(res).await["id"].as_i64().unwrap();

    let res = h
        .client
        .post("/api/projects/3/baselines")
        .header(ContentType::JSON)
        .private_cookie(session(&h.client, LEAD))
        .body(json!({ "name": "CDR" }).to_string())
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let baseline_id = json_of(res).await["id"].as_i64().unwrap();

    let res = h
        .client
        .delete(format!("/api/projects/3/attachments/{id}"))
        .private_cookie(session(&h.client, LEAD))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::NoContent);
    assert_eq!(blob_count(&h), 1, "the baseline keeps the file");

    let res = h
        .client
        .get(format!(
            "/api/projects/3/baselines/{baseline_id}/attachments"
        ))
        .private_cookie(session(&h.client, VIEWER))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let listed = json_of(res).await;
    assert_eq!(listed[0]["id"], id);
    assert_eq!(listed[0]["deleted"], true);

    let res = h
        .client
        .get(format!(
            "/api/projects/3/baselines/{baseline_id}/attachments/{id}/download"
        ))
        .private_cookie(session(&h.client, VIEWER))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.into_bytes().await.unwrap(), bytes);

    let storage = json_of(
        h.client
            .get("/api/projects/3/storage")
            .private_cookie(session(&h.client, LEAD))
            .dispatch()
            .await,
    )
    .await;
    assert_eq!(storage["used_bytes"], 1000);
    assert_eq!(storage["retained_by_baselines_bytes"], 1000);
}
