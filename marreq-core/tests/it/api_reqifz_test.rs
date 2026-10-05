// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! ReqIFZ export and import (issue #343) through the full API router.

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
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

type TestAppState = AppState<CacheRepository<DieselRepoMock>>;

const LEAD: i32 = 1;
const VIEWER: i32 = 2;
const OUTSIDER: i32 = 5;
const SOURCE: i32 = 3;
const TARGET: i32 = 4;
const REQ_A: i32 = 10;
const REQ_B: i32 = 11;
const MAX_ARCHIVE: u64 = 200_000;

const PDF: &[u8] = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n1 0 obj\n<<>>\nendobj\n";
const PNG: &[u8] =
    b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01\0\0\0\x01\x08\x06\0\0\0\x1f\x15\xc4\x89";

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

fn requirement(id: i32, code: &str, title: &str) -> Requirement {
    Requirement {
        id,
        current_version_id: Some(id * 10),
        same_as_current: None,
        title: title.into(),
        description: format!("{title} shall be verified."),
        status_id: SOURCE * 10,
        author_id: LEAD,
        reviewer_id: LEAD,
        reference_code: code.into(),
        category_id: SOURCE * 10,
        parent_id: None,
        creation_date: timestamp(),
        update_date: timestamp(),
        deadline_date: None,
        applicability_id: SOURCE * 10,
        justification: None,
        project_id: SOURCE,
        approval_state: "draft".into(),
        approved_by: None,
        approved_at: None,
        custom_fields: None,
    }
}

/// Status, category, applicability and verification method for a project
/// (the ReqIF import uses the first of each). Ids are `project * 10`.
fn add_catalog(repo: &mut DieselRepoMock, project_id: i32) {
    let id = project_id * 10;
    repo.requirement_statuses.insert(
        id,
        RequirementStatus {
            id,
            title: "Draft".into(),
            description: String::new(),
            tag: "D".into(),
            project_id,
            is_system: true,
            tag_color: None,
        },
    );
    repo.categories.insert(
        id,
        Category {
            id,
            title: "Default".into(),
            description: String::new(),
            tag: "DEF".into(),
            project_id,
        },
    );
    repo.applicability.insert(
        id,
        Applicability {
            id,
            title: "All".into(),
            description: String::new(),
            tag: "ALL".into(),
            project_id,
        },
    );
    repo.verification_methods.insert(
        id,
        VerificationMethod {
            id,
            title: "Test".into(),
            description: String::new(),
            tag: "T".into(),
            project_id,
        },
    );
}

fn version(requirement: &Requirement) -> RequirementVersion {
    RequirementVersion {
        id: requirement.id * 10,
        requirement_id: requirement.id,
        title: requirement.title.clone(),
        description: requirement.description.clone(),
        status_id: requirement.status_id,
        author_id: LEAD,
        reviewer_id: LEAD,
        category_id: requirement.category_id,
        applicability_id: requirement.applicability_id,
        justification: None,
        deadline_date: None,
        created_at: timestamp(),
        approval_state: "draft".into(),
        approved_by: None,
        approved_at: None,
        reviewed_by: None,
        reviewed_at: None,
    }
}

fn repo() -> DieselRepoMock {
    let mut repo = DieselRepoMock::default();
    repo.users
        .insert(LEAD, DieselRepoMock::make_user(LEAD, "lead", "password"));
    repo.users.insert(
        VIEWER,
        DieselRepoMock::make_user(VIEWER, "viewer", "password"),
    );
    repo.users.insert(
        OUTSIDER,
        DieselRepoMock::make_user(OUTSIDER, "outsider", "password"),
    );
    for (id, slug) in [(SOURCE, "lunar-lander"), (TARGET, "copy")] {
        repo.projects.insert(id, project(id, slug));
        add_catalog(&mut repo, id);
        for (user_id, role) in [(LEAD, ROLE_ADMIN), (VIEWER, ROLE_VIEWER)] {
            repo.project_members.push(ProjectMember {
                project_id: id,
                user_id,
                role,
                created_at: timestamp(),
                updated_at: timestamp(),
            });
        }
    }
    for req in [
        requirement(REQ_A, "REQ-THM-001", "Radiator rejects 40 W"),
        requirement(REQ_B, "REQ-THM-002", "Heater keeps 0 C"),
    ] {
        repo.requirement_versions.insert(req.id * 10, version(&req));
        repo.requirements.insert(req.id, req);
    }
    repo.next_version_id = 1000;
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
    let dir = std::env::temp_dir().join(format!("marreq-reqifz-it-{suffix:016x}"));
    let storage = Arc::new(AttachmentStorage::local(AttachmentsConfig {
        dir: dir.clone(),
        max_file_bytes: 50_000,
        default_project_quota_bytes: 10_000_000,
        max_reqifz_bytes: MAX_ARCHIVE,
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

const BOUNDARY: &str = "marreq-reqifz-boundary";

fn multipart(fields: &[(&str, &str)], filename: &str, bytes: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, value) in fields {
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

fn form_type() -> ContentType {
    ContentType::new("multipart", "form-data").with_params(("boundary", BOUNDARY))
}

async fn attach(h: &Harness, requirement_id: i32, filename: &str, bytes: &[u8]) -> i64 {
    let res = h
        .client
        .post(format!("/api/projects/{SOURCE}/attachments"))
        .header(form_type())
        .private_cookie(session(&h.client, LEAD))
        .body(multipart(
            &[
                ("entity_type", "requirement"),
                ("entity_id", &requirement_id.to_string()),
            ],
            filename,
            bytes,
        ))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Created);
    let body: Value = res.into_json().await.unwrap();
    body["id"].as_i64().unwrap()
}

async fn import<'c>(
    h: &'c Harness,
    user_id: i32,
    filename: &str,
    bytes: &[u8],
) -> LocalResponse<'c> {
    h.client
        .post(format!("/api/projects/{TARGET}/imports/reqif"))
        .header(form_type())
        .private_cookie(session(&h.client, user_id))
        .body(multipart(&[], filename, bytes))
        .dispatch()
        .await
}

async fn get_bytes(h: &Harness, path: &str) -> (Status, Option<String>, Vec<u8>) {
    let res = h
        .client
        .get(path)
        .private_cookie(session(&h.client, LEAD))
        .dispatch()
        .await;
    let status = res.status();
    let ct = res.content_type().map(|c| c.to_string());
    (status, ct, res.into_bytes().await.unwrap_or_default())
}

/// (entry name, bytes) of every file in an archive.
fn entries(archive: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive)).expect("valid zip");
    (0..zip.len())
        .map(|i| {
            let mut f = zip.by_index(i).unwrap();
            let mut bytes = Vec::new();
            f.read_to_end(&mut bytes).unwrap();
            (f.name().to_string(), bytes)
        })
        .collect()
}

fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut out);
        for (name, bytes) in files {
            // Stored, so an archive's size is the size of its files.
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file(*name, opts).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }
    out.into_inner()
}

/// A one-object ReqIF document whose statement references `data`.
fn reqif_with_objects(id: &str, title: &str, data: &[&str]) -> String {
    let objects: String = data
        .iter()
        .map(|d| {
            format!(
                "<xhtml:object data=\"{d}\" type=\"application/octet-stream\">{d}</xhtml:object>"
            )
        })
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<REQ-IF xmlns="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd" xmlns:xhtml="http://www.w3.org/1999/xhtml">
  <CORE-CONTENT><REQ-IF-CONTENT>
    <SPEC-TYPES>
      <SPEC-OBJECT-TYPE IDENTIFIER="t">
        <SPEC-ATTRIBUTES>
          <ATTRIBUTE-DEFINITION-STRING IDENTIFIER="ad-title" LONG-NAME="Title"/>
          <ATTRIBUTE-DEFINITION-XHTML IDENTIFIER="ad-text" LONG-NAME="ReqIF.Text"/>
        </SPEC-ATTRIBUTES>
      </SPEC-OBJECT-TYPE>
    </SPEC-TYPES>
    <SPEC-OBJECTS>
      <SPEC-OBJECT IDENTIFIER="{id}" LONG-NAME="{title}">
        <VALUES>
          <ATTRIBUTE-VALUE-STRING THE-VALUE="{title}"><DEFINITION><ATTRIBUTE-DEFINITION-STRING-REF>ad-title</ATTRIBUTE-DEFINITION-STRING-REF></DEFINITION></ATTRIBUTE-VALUE-STRING>
          <ATTRIBUTE-VALUE-XHTML><DEFINITION><ATTRIBUTE-DEFINITION-XHTML-REF>ad-text</ATTRIBUTE-DEFINITION-XHTML-REF></DEFINITION>
            <THE-VALUE><xhtml:div><xhtml:p>Statement of {title}.</xhtml:p><xhtml:p>{objects}</xhtml:p></xhtml:div></THE-VALUE>
          </ATTRIBUTE-VALUE-XHTML>
        </VALUES>
        <TYPE><SPEC-OBJECT-TYPE-REF>t</SPEC-OBJECT-TYPE-REF></TYPE>
      </SPEC-OBJECT>
    </SPEC-OBJECTS>
  </REQ-IF-CONTENT></CORE-CONTENT>
</REQ-IF>"#
    )
}

async fn target_attachments(h: &Harness) -> Vec<Value> {
    let repo_ids: Vec<i32> = {
        let state = h.client.rocket().state::<TestAppState>().unwrap();
        let repo = state.repo.read().unwrap();
        let mut ids: Vec<i32> = repo
            .inner_repo()
            .requirements
            .values()
            .filter(|r| r.project_id == TARGET)
            .map(|r| r.id)
            .collect();
        ids.sort();
        ids
    };
    let mut all = Vec::new();
    for id in repo_ids {
        let res = h
            .client
            .get(format!(
                "/api/projects/{TARGET}/attachments?entity_type=requirement&entity_id={id}"
            ))
            .private_cookie(session(&h.client, LEAD))
            .dispatch()
            .await;
        let list: Vec<Value> = res.into_json().await.unwrap();
        all.extend(list);
    }
    all
}

fn tmp_files(h: &Harness) -> usize {
    std::fs::read_dir(h.dir.join("tmp"))
        .map(|d| d.count())
        .unwrap_or(0)
}

#[rocket::async_test]
async fn export_round_trips_requirements_and_files() {
    let h = harness().await;
    let pdf_id = attach(&h, REQ_A, "TVAC report.pdf", PDF).await;
    let png_id = attach(&h, REQ_A, "plot.png", PNG).await;

    let (status, ct, archive) = get_bytes(
        &h,
        &format!("/api/projects/{SOURCE}/exports/requirements.reqifz"),
    )
    .await;
    assert_eq!(status, Status::Ok);
    assert_eq!(ct.as_deref(), Some("application/zip"));
    assert!(archive.starts_with(b"PK\x03\x04"));

    let files = entries(&archive);
    let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        [
            "requirements-project-3.reqif",
            &*format!("files/{pdf_id}/TVAC report.pdf"),
            &*format!("files/{png_id}/plot.png"),
        ]
    );
    let xml = String::from_utf8(files[0].1.clone()).unwrap();
    assert!(xml.contains(&format!("data=\"files/{pdf_id}/TVAC%20report.pdf\"")));
    assert!(xml.contains(&format!("data=\"files/{png_id}/plot.png\"")));
    assert_eq!(files[1].1, PDF);

    let res = import(&h, LEAD, "requirements-project-3.reqifz", &archive).await;
    assert_eq!(res.status(), Status::Ok);
    let body: Value = res.into_json().await.unwrap();
    assert_eq!(body["success"], true, "{body}");
    assert_eq!(body["imported_count"], 2);
    assert_eq!(body["imported_attachment_count"], 2);
    assert_eq!(body["documents"], json!(["requirements-project-3.reqif"]));
    assert!(
        !body["warnings"].as_array().unwrap().iter().any(|w| {
            let w = w.as_str().unwrap();
            w.contains("embedded") || w.contains("Attachments")
        }),
        "{body}"
    );

    let copied = target_attachments(&h).await;
    let mut copied_names: Vec<&str> = copied
        .iter()
        .map(|a| a["filename"].as_str().unwrap())
        .collect();
    copied_names.sort();
    assert_eq!(copied_names, ["TVAC report.pdf", "plot.png"]);
    let pdf = copied
        .iter()
        .find(|a| a["filename"] == "TVAC report.pdf")
        .unwrap();
    assert_eq!(pdf["content_type"], "application/pdf");
    let (_, _, bytes) = get_bytes(
        &h,
        &format!("/api/projects/{TARGET}/attachments/{}/download", pdf["id"]),
    )
    .await;
    assert_eq!(bytes, PDF);
    assert_eq!(tmp_files(&h), 0);
}

#[rocket::async_test]
async fn baseline_export_includes_files_deleted_since() {
    let h = harness().await;
    let pdf_id = attach(&h, REQ_B, "spec.pdf", PDF).await;
    let res = h
        .client
        .post(format!("/api/projects/{SOURCE}/baselines"))
        .header(ContentType::JSON)
        .private_cookie(session(&h.client, LEAD))
        .body(json!({ "name": "CDR" }).to_string())
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let baseline: Value = res.into_json().await.unwrap();
    let bid = baseline["id"].as_i64().unwrap();
    let res = h
        .client
        .delete(format!("/api/projects/{SOURCE}/attachments/{pdf_id}"))
        .private_cookie(session(&h.client, LEAD))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::NoContent);

    let (status, ct, archive) = get_bytes(
        &h,
        &format!("/api/projects/{SOURCE}/exports/baselines/{bid}.reqifz"),
    )
    .await;
    assert_eq!(status, Status::Ok);
    assert_eq!(ct.as_deref(), Some("application/zip"));
    let names: Vec<String> = entries(&archive).into_iter().map(|(n, _)| n).collect();
    assert!(
        names.contains(&format!("files/{pdf_id}/spec.pdf")),
        "{names:?}"
    );

    // The plain baseline export still works on the same route.
    let (status, ct, xml) = get_bytes(
        &h,
        &format!("/api/projects/{SOURCE}/exports/baselines/{bid}.reqif"),
    )
    .await;
    assert_eq!(status, Status::Ok);
    assert_eq!(ct.as_deref(), Some("application/xml"));
    assert!(!String::from_utf8(xml).unwrap().contains("xhtml:object"));
    let (status, _, _) = get_bytes(
        &h,
        &format!("/api/projects/{SOURCE}/exports/baselines/{bid}.zip"),
    )
    .await;
    assert_eq!(status, Status::NotFound);
}

#[rocket::async_test]
async fn imports_every_document_and_skips_files_that_fail_checks() {
    let h = harness().await;
    let first = reqif_with_objects(
        "obj-1",
        "Battery sizing",
        &[
            "files/sizing.csv",
            "files/logo.svg",
            "files/missing.pdf",
            "../outside.pdf",
        ],
    );
    let second = reqif_with_objects("obj-2", "Solar array", &["../shared/array.png"]);
    let archive = zip_of(&[
        ("power/battery.reqif", first.as_bytes()),
        ("power/files/sizing.csv", b"cell,Ah\n1,3.2\n"),
        (
            "power/files/logo.svg",
            b"<svg xmlns='http://www.w3.org/2000/svg'/>",
        ),
        ("solar/array.reqif", second.as_bytes()),
        ("shared/array.png", PNG),
    ]);
    let res = import(&h, LEAD, "power.reqifz", &archive).await;
    assert_eq!(res.status(), Status::Ok);
    let body: Value = res.into_json().await.unwrap();
    assert_eq!(body["imported_count"], 2, "{body}");
    assert_eq!(body["imported_attachment_count"], 2, "{body}");
    assert_eq!(
        body["documents"],
        json!(["power/battery.reqif", "solar/array.reqif"])
    );
    let warnings: Vec<String> = body["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w.as_str().unwrap().to_string())
        .collect();
    let has = |needle: &str| warnings.iter().any(|w| w.contains(needle));
    assert!(
        has("power/battery.reqif: Battery sizing: logo.svg"),
        "{warnings:?}"
    );
    assert!(
        has("files/missing.pdf is not in the archive"),
        "{warnings:?}"
    );
    assert!(has("../outside.pdf is not in the archive"), "{warnings:?}");

    let copied = target_attachments(&h).await;
    let mut names: Vec<&str> = copied
        .iter()
        .map(|a| a["filename"].as_str().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, ["array.png", "sizing.csv"]);
    assert_eq!(tmp_files(&h), 0, "skipped files leave nothing behind");
}

#[rocket::async_test]
async fn rejects_unsafe_oversized_and_unauthorised_archives() {
    let h = harness().await;
    let doc = reqif_with_objects("obj-1", "Evil", &[]);
    let evil = zip_of(&[
        ("doc.reqif", doc.as_bytes()),
        ("../../etc/cron.d/x", b"boom"),
    ]);
    let res = import(&h, LEAD, "evil.reqifz", &evil).await;
    assert_eq!(res.status(), Status::BadRequest);
    let body: Value = res.into_json().await.unwrap();
    assert!(
        body["message"].as_str().unwrap().contains("unsafe path"),
        "{body}"
    );
    assert!(target_attachments(&h).await.is_empty());

    let big = zip_of(&[
        ("doc.reqif", doc.as_bytes()),
        ("files/big.bin", &vec![1u8; MAX_ARCHIVE as usize]),
    ]);
    let res = import(&h, LEAD, "big.reqifz", &big).await;
    assert_eq!(res.status(), Status::PayloadTooLarge);

    let ok = zip_of(&[("doc.reqif", doc.as_bytes())]);
    let res = import(&h, VIEWER, "ok.reqifz", &ok).await;
    assert_eq!(res.status(), Status::Forbidden);

    let res = import(&h, LEAD, "fake.reqifz", doc.as_bytes()).await;
    assert_eq!(res.status(), Status::BadRequest);
    let body: Value = res.into_json().await.unwrap();
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("not a ZIP archive"),
        "{body}"
    );
}

#[rocket::async_test]
async fn imports_the_polarion_reqifz_fixture() {
    let h = harness().await;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/reqif/fixtures/polarion/polarion.reqifz");
    let archive = std::fs::read(path).expect("fixture");
    let res = import(&h, LEAD, "polarion.reqifz", &archive).await;
    assert_eq!(res.status(), Status::Ok);
    let body: Value = res.into_json().await.unwrap();
    assert!(body["imported_count"].as_u64().unwrap() > 0, "{body}");
    assert_eq!(body["documents"], json!(["sample1_polarion.reqif"]));
}

#[rocket::async_test]
async fn export_requires_view_permission() {
    let h = harness().await;
    let res = h
        .client
        .get(format!(
            "/api/projects/{SOURCE}/exports/requirements.reqifz"
        ))
        .private_cookie(session(&h.client, OUTSIDER))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Forbidden);
}
