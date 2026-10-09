// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Dependency Structure Matrix (issue #328) through the full API router.

#![cfg(feature = "test-helpers")]

use calamine::{Data, Reader, open_workbook_auto_from_rs};
use chrono::{NaiveDate, NaiveDateTime};
use marreq_core::app::AppState;
use marreq_core::auth::session::test_session_cookie_for;
use marreq_core::models::*;
use marreq_core::permissions::ROLE_VIEWER;
use marreq_core::repository::{CacheRepository, diesel_repo_mock::DieselRepoMock};
use marreq_core::status_enums::ProjectStatus;
use rocket::http::{Cookie, Status};
use rocket::local::asynchronous::Client;
use serde_json::Value;
use std::io::Cursor;
use std::sync::{Arc, RwLock};

type TestAppState = AppState<CacheRepository<DieselRepoMock>>;

const VIEWER: i32 = 2;
const OUTSIDER: i32 = 3;

fn day(d: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, d)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

fn requirement(id: i32, code: &str, category_id: i32) -> Requirement {
    Requirement {
        id,
        current_version_id: Some(id * 10),
        same_as_current: None,
        title: format!("Requirement {code}"),
        description: String::new(),
        status_id: 1,
        author_id: 1,
        reviewer_id: 1,
        reference_code: code.into(),
        category_id,
        parent_id: None,
        creation_date: day(1),
        update_date: day(1),
        deadline_date: None,
        applicability_id: 1,
        justification: None,
        project_id: 1,
        approval_state: "draft".into(),
        approved_by: None,
        approved_at: None,
        custom_fields: None,
    }
}

fn version(id: i32, requirement_id: i32) -> RequirementVersion {
    RequirementVersion {
        id,
        requirement_id,
        title: String::new(),
        description: String::new(),
        status_id: 1,
        author_id: 1,
        reviewer_id: 1,
        category_id: 1,
        applicability_id: 1,
        justification: None,
        deadline_date: None,
        created_at: day(1),
        approval_state: "draft".into(),
        approved_by: None,
        approved_at: None,
        reviewed_by: None,
        reviewed_at: None,
    }
}

fn link(id: i32, source: i32, target: i32, link_type: &str) -> RequirementVersionLink {
    RequirementVersionLink {
        id,
        source_version_id: source * 10,
        target_version_id: target * 10,
        link_type: link_type.into(),
        rationale: None,
        project_id: 1,
        created_at: day(1) + chrono::Duration::seconds(id as i64),
        metadata: None,
    }
}

/// COMM-1 ← COMM-2 ← COMM-3 ← COMM-2 (loop), PWR-1 approved before COMM-1 changed.
fn repo() -> DieselRepoMock {
    let mut repo = DieselRepoMock::default();
    let mut admin = DieselRepoMock::make_user(1, "admin", "password");
    admin.is_admin = true;
    repo.users.insert(1, admin);
    repo.users.insert(
        VIEWER,
        DieselRepoMock::make_user(VIEWER, "viewer", "password"),
    );
    repo.users.insert(
        OUTSIDER,
        DieselRepoMock::make_user(OUTSIDER, "outsider", "password"),
    );
    repo.projects.insert(
        1,
        Project {
            id: 1,
            name: "Space".into(),
            description: None,
            creation_date: None,
            update_date: None,
            status: ProjectStatus::Active,
            owner_id: Some(1),
            slug: "space".into(),
            group_id: None,
            archived_at: None,
            archived_by: None,
        },
    );
    repo.project_members.push(ProjectMember {
        project_id: 1,
        user_id: VIEWER,
        role: ROLE_VIEWER,
        created_at: day(1),
        updated_at: day(1),
    });
    for (id, title) in [(1, "Power"), (2, "Communications")] {
        repo.categories.insert(
            id,
            Category {
                id,
                title: title.into(),
                description: String::new(),
                tag: String::new(),
                project_id: 1,
            },
        );
    }
    let mut reqs = vec![
        requirement(1, "COMM-1", 2),
        requirement(2, "COMM-2", 2),
        requirement(3, "COMM-3", 2),
        requirement(4, "PWR-1", 1),
    ];
    reqs[0].update_date = day(20); // COMM-1 edited after PWR-1's approval
    reqs[3].approval_state = "approved".into();
    reqs[3].approved_at = Some(day(10));
    for r in reqs {
        repo.requirement_versions
            .insert(r.id * 10, version(r.id * 10, r.id));
        repo.requirements.insert(r.id, r);
    }
    repo.requirement_version_links = vec![
        link(1, 2, 1, "DERIVES_FROM"),
        link(2, 3, 2, "DEPENDS_ON"),
        link(3, 2, 3, "DEPENDS_ON"),
        link(4, 4, 1, "SATISFIES"),
    ];
    repo.next_link_id = 5;
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

fn index_of(body: &Value, code: &str) -> u64 {
    body["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["reference_code"] == code)
        .unwrap()["index"]
        .as_u64()
        .unwrap()
}

#[rocket::async_test]
async fn dsm_lists_marks_loops_and_upstream_changes_for_a_viewer() {
    let client = client().await;
    let res = client
        .get("/api/projects/1/dsm")
        .private_cookie(session(&client, VIEWER))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let body: Value = res.into_json().await.unwrap();

    // Categories by name: Communications block first, then Power.
    let groups: Vec<&str> = body["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["label"].as_str().unwrap())
        .collect();
    assert_eq!(groups, ["Communications", "Power"]);

    let cells = body["cells"].as_array().unwrap();
    assert_eq!(cells.len(), 4);
    let find = |from: &str, to: &str| {
        let (r, c) = (index_of(&body, from), index_of(&body, to));
        cells
            .iter()
            .find(|cell| cell["row"] == r && cell["col"] == c)
            .cloned()
    };
    assert_eq!(
        find("COMM-2", "COMM-1").unwrap()["link_types"][0],
        "DERIVES_FROM"
    );
    assert_eq!(find("COMM-2", "COMM-3").unwrap()["in_loop"], true);
    assert_eq!(find("PWR-1", "COMM-1").unwrap()["upstream_changed"], true);

    let loops = body["loops"].as_array().unwrap();
    assert_eq!(loops.len(), 1);
    assert_eq!(loops[0]["requirement_ids"].as_array().unwrap().len(), 2);
}

#[rocket::async_test]
async fn dsm_forbids_non_members() {
    let client = client().await;
    let res = client
        .get("/api/projects/1/dsm")
        .private_cookie(session(&client, OUTSIDER))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Forbidden);
}

#[rocket::async_test]
async fn dsm_export_returns_workbook_with_matrix_loops_and_legend() {
    let client = client().await;
    let res = client
        .get("/api/projects/1/exports/dsm.xlsx?order=partition")
        .private_cookie(session(&client, VIEWER))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(
        res.headers().get_one("Content-Disposition"),
        Some("attachment; filename=\"dsm-project-1.xlsx\"")
    );
    let bytes = res.into_bytes().await.expect("body");
    let mut workbook = open_workbook_auto_from_rs(Cursor::new(bytes)).expect("xlsx");
    assert_eq!(workbook.sheet_names(), ["DSM", "Loops", "Legend"]);

    let dsm = workbook.worksheet_range("DSM").unwrap();
    let text = |row: usize, col: usize| match dsm.get((row, col)) {
        Some(Data::String(s)) => s.clone(),
        Some(other) => other.to_string(),
        None => String::new(),
    };
    // Row headers are in column B; column headers (codes) in row 1 from column E.
    let codes: Vec<String> = (1..=4).map(|r| text(r, 1)).collect();
    let col_of = |code: &str| 4 + codes.iter().position(|c| c == code).unwrap();
    let row_of = |code: &str| 1 + codes.iter().position(|c| c == code).unwrap();
    assert_eq!(text(0, col_of("COMM-1")), "COMM-1");
    assert_eq!(text(row_of("COMM-2"), col_of("COMM-1")), "D");
    assert_eq!(text(row_of("PWR-1"), col_of("COMM-1")), "S");
    assert_eq!(text(row_of("COMM-1"), col_of("COMM-2")), "");

    let loops = workbook.worksheet_range("Loops").unwrap();
    assert_eq!(loops.height(), 2, "header + one loop");
}

#[rocket::async_test]
async fn dsm_export_rejects_unknown_link_type() {
    let client = client().await;
    let res = client
        .get("/api/projects/1/exports/dsm.xlsx?link_types=BOGUS")
        .private_cookie(session(&client, VIEWER))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::BadRequest);
}
