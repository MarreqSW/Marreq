// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Project bundle with attachment files, `bundle.zip` (issue #341).

#![cfg(feature = "test-helpers")]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use marreq_core::storage::{AttachmentStorage, AttachmentsConfig};
use rocket::http::{ContentType, Status};
use rocket::local::asynchronous::{Client, LocalResponse};
use serde_json::{Value, json};

use crate::api_project_bundle_test::test_support::{bundle_repo, managed_state, session_cookie};

const ADMIN: i32 = 1;
const VIEWER: i32 = 2;
const BOUNDARY: &str = "----MarreqBundleFilesBoundary";
const PDF_HEAD: &[u8] = b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n";

fn pdf(size: usize, fill: u8) -> Vec<u8> {
    let mut bytes = PDF_HEAD.to_vec();
    bytes.resize(size, fill);
    bytes
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

async fn harness(max_file_bytes: u64, quota_bytes: u64) -> Harness {
    marreq_core::deployment::install_test_server_mode();
    let suffix: u64 = rand::random();
    let dir = std::env::temp_dir().join(format!("marreq-bundle-files-it-{suffix:016x}"));
    let storage = Arc::new(AttachmentStorage::local(AttachmentsConfig {
        dir: dir.clone(),
        max_file_bytes,
        default_project_quota_bytes: quota_bytes,
        ..AttachmentsConfig::default()
    }));
    let rocket = rocket::build()
        .manage(managed_state(bundle_repo()))
        .manage(storage)
        .manage(marreq_core::auth::AuthConfig::default())
        .manage(marreq_core::auth::rate_limiter::LoginRateLimiter::new())
        .mount("/api", marreq_core::api::routes());
    Harness {
        client: Client::tracked(rocket).await.expect("rocket instance"),
        dir,
    }
}

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

fn form() -> ContentType {
    ContentType::new("multipart", "form-data").with_params(("boundary", BOUNDARY))
}

async fn attach(h: &Harness, entity: (&str, i32), filename: &str, bytes: &[u8]) {
    let id = entity.1.to_string();
    let res = h
        .client
        .post("/api/projects/1/attachments")
        .header(form())
        .private_cookie(session_cookie(&h.client, ADMIN))
        .body(multipart(
            &[("entity_type", entity.0), ("entity_id", &id)],
            filename,
            bytes,
        ))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Created, "upload {filename}");
}

async fn export_zip(h: &Harness, user: i32) -> LocalResponse<'_> {
    h.client
        .get("/api/projects/1/exports/bundle.zip")
        .private_cookie(session_cookie(&h.client, user))
        .dispatch()
        .await
}

async fn import<'c>(h: &'c Harness, filename: &str, bytes: &[u8]) -> LocalResponse<'c> {
    h.client
        .post("/api/projects/imports/bundle")
        .header(form())
        .private_cookie(session_cookie(&h.client, ADMIN))
        .body(multipart(&[], filename, bytes))
        .dispatch()
        .await
}

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
            let opts = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            zip.start_file(*name, opts).unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
    }
    out.into_inner()
}

/// The attachments of one imported requirement or verification, with their bytes.
async fn files_of(h: &Harness, project: i64, entity: &str, id: i64) -> Vec<(String, Vec<u8>)> {
    let listed: Value = h
        .client
        .get(format!(
            "/api/projects/{project}/attachments?entity_type={entity}&entity_id={id}"
        ))
        .private_cookie(session_cookie(&h.client, ADMIN))
        .dispatch()
        .await
        .into_json()
        .await
        .expect("attachment list");
    let mut out = Vec::new();
    for a in listed.as_array().expect("array") {
        let bytes = h
            .client
            .get(format!(
                "/api/projects/{project}/attachments/{}/download",
                a["id"]
            ))
            .private_cookie(session_cookie(&h.client, ADMIN))
            .dispatch()
            .await
            .into_bytes()
            .await
            .unwrap();
        out.push((a["filename"].as_str().unwrap().to_string(), bytes));
    }
    out
}

/// Id of the imported entity with `reference_code` in `project`.
async fn id_by_code(h: &Harness, project: i64, path: &str, code: &str) -> i64 {
    let listed: Value = h
        .client
        .get(format!("/api/projects/{project}/{path}"))
        .private_cookie(session_cookie(&h.client, ADMIN))
        .dispatch()
        .await
        .into_json()
        .await
        .expect("list");
    listed
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["reference_code"] == code)
        .and_then(|r| r["id"].as_i64())
        .unwrap_or_else(|| panic!("{code} not in project {project}"))
}

fn store_tmp_is_empty(h: &Harness) -> bool {
    std::fs::read_dir(h.dir.join("tmp")).map_or(true, |mut d| d.next().is_none())
}

#[rocket::async_test]
async fn bundle_zip_round_trips_requirement_and_verification_files() {
    let h = harness(1024 * 1024, 10 * 1024 * 1024).await;
    let spec = pdf(3000, b's');
    let report = pdf(2000, b'r');
    attach(&h, ("requirement", 1), "spec.pdf", &spec).await;
    attach(&h, ("verification", 1), "test report.pdf", &report).await;

    // Viewers may export; the JSON-only bundle stays without files.
    let res = export_zip(&h, VIEWER).await;
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(res.content_type(), Some(ContentType::ZIP));
    assert_eq!(
        res.headers().get_one("Content-Disposition"),
        Some("attachment; filename=\"project-satellite-demo-bundle.zip\"")
    );
    let archive = res.into_bytes().await.unwrap();
    let files = entries(&archive);
    assert_eq!(files[0].0, "bundle.json", "the bundle document comes first");
    let bundle: Value = serde_json::from_slice(&files[0].1).unwrap();
    let listed = bundle["attachments"].as_array().expect("attachments");
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0]["entity_type"], "requirement");
    assert_eq!(listed[0]["entity_reference_code"], "SAT-PWR-001");
    assert_eq!(listed[1]["entity_type"], "verification");
    for a in listed {
        let path = a["path"].as_str().unwrap();
        assert!(
            files.iter().any(|(name, _)| name == path),
            "{path} in archive"
        );
    }
    let json_only: Value = h
        .client
        .get("/api/projects/1/exports/bundle.json")
        .private_cookie(session_cookie(&h.client, VIEWER))
        .dispatch()
        .await
        .into_json()
        .await
        .unwrap();
    assert!(json_only.get("attachments").is_none());

    // Import as a new project: the files come back on the right entities.
    let res = import(&h, "project-satellite-demo-bundle.zip", &archive).await;
    assert_eq!(res.status(), Status::Ok);
    let result: Value = res.into_json().await.unwrap();
    assert_eq!(result["imported_counts"]["attachments"], 2);
    assert_eq!(result["warnings"], json!([]));
    let project = result["project_id"].as_i64().unwrap();
    assert_ne!(project, 1);

    let req = id_by_code(&h, project, "requirements", "SAT-PWR-001").await;
    assert_eq!(
        files_of(&h, project, "requirement", req).await,
        vec![("spec.pdf".to_string(), spec)]
    );
    let ver = id_by_code(&h, project, "verifications", "VER-PWR-001").await;
    assert_eq!(
        files_of(&h, project, "verification", ver).await,
        vec![("test report.pdf".to_string(), report)]
    );
    assert!(store_tmp_is_empty(&h));
}

#[rocket::async_test]
async fn files_that_do_not_fit_are_skipped_with_warnings() {
    // 4 KiB per file, 6 KiB per project.
    let h = harness(4096, 6144).await;
    attach(&h, ("requirement", 1), "ok.pdf", &pdf(3000, b'a')).await;
    let archive = export_zip(&h, ADMIN).await.into_bytes().await.unwrap();
    let files = entries(&archive);
    let mut bundle: Value = serde_json::from_slice(&files[0].1).unwrap();
    let ok_path = bundle["attachments"][0]["path"]
        .as_str()
        .unwrap()
        .to_string();
    let extra = |code: &str, path: &str, filename: &str| {
        json!({
            "entity_type": "requirement",
            "entity_reference_code": code,
            "filename": filename,
            "content_type": "application/pdf",
            "path": path,
        })
    };
    let list = bundle["attachments"].as_array_mut().unwrap();
    list.push(extra("SAT-PWR-001", "files/90/huge.pdf", "huge.pdf"));
    list.push(extra("SAT-PWR-001", "files/91/tool.exe", "tool.exe"));
    list.push(extra("SAT-PWR-001", "files/92/over.pdf", "over.pdf"));
    list.push(extra("SAT-PWR-001", "files/93/missing.pdf", "missing.pdf"));
    list.push(extra("NO-SUCH-REQ", &ok_path, "ok.pdf"));
    let ok = files.iter().find(|(n, _)| *n == ok_path).unwrap().1.clone();
    let doc = serde_json::to_vec(&bundle).unwrap();
    let crafted = zip_of(&[
        ("bundle.json", &doc),
        (&ok_path, &ok),
        ("files/90/huge.pdf", &pdf(5000, b'h')),
        ("files/91/tool.exe", b"MZ\x90\x00 not allowed"),
        ("files/92/over.pdf", &pdf(4000, b'o')),
    ]);

    let res = import(&h, "crafted.zip", &crafted).await;
    assert_eq!(res.status(), Status::Ok);
    let result: Value = res.into_json().await.unwrap();
    assert_eq!(result["imported_counts"]["requirements"], 1);
    assert_eq!(result["imported_counts"]["attachments"], 1, "{result}");
    let warnings: Vec<&str> = result["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w.as_str().unwrap())
        .collect();
    let has = |needle: &str| warnings.iter().any(|w| w.contains(needle));
    assert!(has("huge.pdf is larger than"), "{warnings:?}");
    assert!(has(".exe files are not allowed"), "{warnings:?}");
    assert!(has("over.pdf"), "quota: {warnings:?}");
    assert!(has("missing.pdf"), "{warnings:?}");
    assert!(has("NO-SUCH-REQ"), "{warnings:?}");
    assert!(store_tmp_is_empty(&h));
}

#[rocket::async_test]
async fn unusable_archives_are_rejected() {
    let h = harness(4096, 1024 * 1024).await;
    let no_bundle = zip_of(&[("files/1/a.pdf", &pdf(100, b'a'))]);
    let res = import(&h, "no-bundle.zip", &no_bundle).await;
    assert_eq!(res.status(), Status::BadRequest);
    let body: Value = res.into_json().await.unwrap();
    assert!(body["message"].as_str().unwrap().contains("no bundle.json"));

    let unsafe_path = zip_of(&[("bundle.json", b"{}"), ("../escape.pdf", b"x")]);
    let res = import(&h, "unsafe.zip", &unsafe_path).await;
    assert_eq!(res.status(), Status::BadRequest);

    let bad_json = zip_of(&[(
        "bundle.json",
        br#"{"format":"nope","project":{"name":"X"}}"#,
    )]);
    let res = import(&h, "bad.zip", &bad_json).await;
    assert_eq!(res.status(), Status::BadRequest);
}

#[rocket::async_test]
async fn json_bundle_listing_files_imports_with_a_warning() {
    let h = harness(4096, 1024 * 1024).await;
    attach(&h, ("requirement", 1), "a.pdf", &pdf(500, b'a')).await;
    let archive = export_zip(&h, ADMIN).await.into_bytes().await.unwrap();
    let doc = entries(&archive).remove(0).1;
    let res = import(&h, "bundle.json", &doc).await;
    assert_eq!(res.status(), Status::Ok);
    let result: Value = res.into_json().await.unwrap();
    assert_eq!(result["imported_counts"]["attachments"], 0);
    assert!(
        result["warnings"][0]
            .as_str()
            .unwrap()
            .contains("import the .zip bundle")
    );
}
