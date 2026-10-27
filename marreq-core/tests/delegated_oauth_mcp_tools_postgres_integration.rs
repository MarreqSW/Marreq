// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Delegated OAuth scope matrix for the project routes the MCP server wraps (#416, E0).
//! Production-path delegated OAuth tests. Set `MARREQ_TEST_DATABASE_URL` to a
//! migrated, disposable PostgreSQL database; the test truncates application data.

#![cfg(not(feature = "test-helpers"))]

use chrono::{Duration, Utc};
use diesel::connection::SimpleConnection;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sql_types::{Array, Integer, Text};
use marreq_core::auth::delegated::hash_secret;
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::Client;

struct TestMode;
impl marreq_core::deployment::DeploymentMode for TestMode {
    fn name(&self) -> &'static str {
        "server"
    }
    fn allows_self_registration(&self) -> bool {
        false
    }
    fn requires_email_verification(&self) -> bool {
        false
    }
    fn allows_admin_promotion(&self) -> bool {
        true
    }
    fn assigns_personal_workspace(&self) -> bool {
        false
    }
}
static TEST_MODE: TestMode = TestMode;

#[derive(QueryableByName)]
struct InsertedId {
    #[diesel(sql_type = Integer)]
    id: i32,
}

fn insert_token(
    conn: &mut PgConnection,
    raw: &str,
    user_id: i32,
    scopes: &[&str],
    expired: bool,
    revoked: bool,
) -> i32 {
    let client_id = format!("client-{raw}");
    diesel::sql_query("INSERT INTO oauth_clients (client_id,name,redirect_uris) VALUES ($1,$2,'[\"http://localhost/callback\"]'::jsonb)")
        .bind::<Text, _>(&client_id).bind::<Text, _>(&client_id).execute(conn).unwrap();
    let scope_values = scopes.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    let grant = diesel::sql_query("INSERT INTO oauth_grants (user_id,client_id,scopes,resource,revoked_at) VALUES ($1,$2,$3,$4,CASE WHEN $5='yes' THEN NOW() ELSE NULL END) RETURNING id")
        .bind::<Integer, _>(user_id).bind::<Text, _>(&client_id)
        .bind::<Array<Text>, _>(&scope_values).bind::<Text, _>("http://localhost:8080/mcp")
        .bind::<Text, _>(if revoked { "yes" } else { "no" })
        .get_result::<InsertedId>(conn).unwrap();
    let expiry = if expired {
        Utc::now().naive_utc() - Duration::minutes(1)
    } else {
        Utc::now().naive_utc() + Duration::hours(1)
    };
    diesel::sql_query("INSERT INTO oauth_access_tokens (token_hash,grant_id,client_id,scopes,resource,expires_at) VALUES ($1,$2,$3,$4,$5,$6)")
        .bind::<Text, _>(hash_secret(raw)).bind::<Integer, _>(grant.id).bind::<Text, _>(&client_id)
        .bind::<Array<Text>, _>(&scope_values).bind::<Text, _>("http://localhost:8080/mcp")
        .bind::<diesel::sql_types::Timestamp, _>(expiry).execute(conn).unwrap();
    grant.id
}

fn bearer(raw: &str) -> Header<'static> {
    Header::new("Authorization", format!("Bearer {raw}"))
}

const RES_BASE: &str = "/api/projects/1";

async fn status_of(
    client: &Client,
    method: &str,
    path: &str,
    token: &str,
    body: Option<serde_json::Value>,
    key: Option<&str>,
) -> (Status, String) {
    let recorded_body = body.clone();
    let mut request = match method {
        "GET" => client.get(path.to_owned()),
        "POST" => client.post(path.to_owned()),
        "DELETE" => client.delete(path.to_owned()),
        "PUT" => client.put(path.to_owned()),
        other => panic!("unsupported method {other}"),
    }
    .header(bearer(token));
    if let Some(key) = key {
        request = request.header(Header::new("Idempotency-Key", key.to_owned()));
    }
    if let Some(body) = body {
        request = request.header(ContentType::JSON).body(body.to_string());
    }
    let response = request.dispatch().await;
    let status = response.status();
    let text = response.into_string().await.unwrap_or_default();
    if let Ok(output) = std::env::var("MARREQ_CONTRACT_TRANSCRIPT") {
        use std::io::Write;
        let record = serde_json::json!({
            "method": method, "path": path, "status": status.code,
            "request": recorded_body,
            "response": serde_json::from_str::<serde_json::Value>(&text).unwrap_or(serde_json::Value::Null),
        });
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(output)
            .expect("contract transcript");
        writeln!(file, "{record}").expect("record actual Rocket exchange");
    }
    (status, text)
}

#[rocket::async_test]
async fn delegated_scopes_gate_the_routes_wrapped_by_mcp_tools() {
    let Ok(database_url) = std::env::var("MARREQ_TEST_DATABASE_URL") else {
        eprintln!(
            "skipping PostgreSQL OAuth integration test: set MARREQ_TEST_DATABASE_URL to a disposable migrated database to run it"
        );
        return;
    };
    // SAFETY: this is the only test in its binary and it runs before any Rocket
    // instance, database connection or other thread that reads the environment exists.
    unsafe {
        std::env::set_var("DATABASE_URL", &database_url);
        std::env::set_var(
            "ROCKET_DATABASES",
            format!(r#"{{my_db={{url="{database_url}",pool_size=2}}}}"#),
        );
        std::env::set_var("MARREQ_PUBLIC_BASE_URL", "http://localhost:8080");
        std::env::set_var("MARREQ_MCP_PUBLIC_URL", "http://localhost:8080/mcp/");
        std::env::set_var(
            "MARREQ_MCP_AUDIT_SECRET",
            "integration-test-audit-secret-32-bytes",
        );
    }
    let mut conn = PgConnection::establish(&database_url).expect("test PostgreSQL connection");
    conn.batch_execute(
        "TRUNCATE users, projects, oauth_clients RESTART IDENTITY CASCADE;
         INSERT INTO users (id,username,name,email,password_hash,is_admin) VALUES
           (1,'manager','Manager','manager@example.test','!',false);
         INSERT INTO projects (id,name,description,status,owner_id,slug) VALUES
           (1,'Project A','A','active',1,'project-a'),
           (2,'Project B','B','active',NULL,'project-b');
         INSERT INTO project_members (project_id,user_id,role) VALUES (1,1,2);
         INSERT INTO project_reviewers (project_id,user_id) VALUES (1,1);
         INSERT INTO requirement_status (id,title,description,tag,project_id,is_system) VALUES (1,'Draft','','DRAFT',1,true),(2,'Draft','','DRAFT',2,true);
         INSERT INTO categories (id,title,description,tag,project_id) VALUES (1,'General','','GEN',1),(2,'General','','GEN',2);
         INSERT INTO applicability (id,title,description,tag,project_id) VALUES (1,'All','','ALL',1),(2,'All','','ALL',2);
         INSERT INTO verification_methods (id,title,description,tag,project_id) VALUES (1,'Analysis','','AN',1);
         INSERT INTO verification_status (id,title,description,tag,project_id,is_system) VALUES (1,'Not run','','NOT_RUN',1,true);
         INSERT INTO requirements (id,project_id,stable_code) VALUES (1,1,'REQ-1'),(2,1,'REQ-2'),(3,2,'REQ-3'),(4,2,'REQ-4');
         INSERT INTO requirement_versions (id,requirement_id,title,description,status_id,author_id,reviewer_id,category_id,applicability_id,approval_state) VALUES
           (1,1,'One','D',1,1,1,1,1,'draft'),(2,2,'Two','D',1,1,1,1,1,'draft'),
           (3,3,'Three','D',2,1,1,2,2,'draft'),(4,4,'Four','D',2,1,1,2,2,'draft');
         UPDATE requirements SET current_version_id=id;
         INSERT INTO requirement_version_links (id,source_version_id,target_version_id,link_type,project_id) VALUES (500,3,4,'DERIVES_FROM',2);
         INSERT INTO verifications (id,name,reference_code,description,source,status_id,project_id,verification_method_id,author_id,reviewer_id)
           VALUES (1,'Verification','VER-1','Description','manual',1,1,1,1,1);
         SELECT setval(pg_get_serial_sequence('requirement_version_links','id'), 500);",
    )
    .expect("seed fixtures");

    for (raw, scopes) in [
        ("projects", vec!["projects:read"]),
        ("uc-read", vec!["use_cases:read"]),
        ("uc-write", vec!["use_cases:read", "use_cases:write"]),
        ("req-read", vec!["requirements:read"]),
        ("trace-read", vec!["traceability:read"]),
        ("trace-write", vec!["traceability:write"]),
        ("ver-read", vec!["verifications:read"]),
        ("ver-base", vec!["verifications:read", "baselines:read"]),
        ("req-ver", vec!["requirements:read", "verifications:read"]),
    ] {
        insert_token(&mut conn, raw, 1, &scopes, false, false);
    }

    marreq_core::config::AppConfig::install_from_env_or_exit();
    let rocket = marreq_core::app::build_with_auth(
        &TEST_MODE,
        marreq_core::auth::AuthConfig::default(),
        vec![],
        vec![],
    );
    let client = Client::tracked(rocket).await.expect("Rocket client");

    // Contract verification uses the real Rocket handlers and migrated PostgreSQL.
    // Credentials never enter the transcript; only synthetic payloads are recorded.
    for path in ["/api/meta/build", "/api/meta/deployment"] {
        let (status, _) = status_of(&client, "GET", path, "uc-read", None, None).await;
        assert_eq!(status, Status::Ok);
    }
    let use_cases = format!("{RES_BASE}/use-cases");
    let (status, _) = status_of(&client, "GET", &use_cases, "req-read", None, None).await;
    assert_eq!(status, Status::Forbidden);
    let (status, created) = status_of(
        &client,
        "POST",
        &use_cases,
        "uc-write",
        Some(serde_json::json!({"title": "Contract use case", "primary_actor": "Tester"})),
        Some("contract-create-use-case"),
    )
    .await;
    assert_eq!(status, Status::Created, "{created}");
    let created: serde_json::Value = serde_json::from_str(&created).unwrap();
    let uc_id = created["id"].as_i64().unwrap();
    let revision = created["draft"]["revision"].as_i64().unwrap();
    let draft_path = format!("{use_cases}/{uc_id}/draft");
    let document = serde_json::json!({
        "schema": "marreq.use-case.v1", "title": "Contract use case",
        "main": [{"id": "s_test1", "text": "Verify the API contract"}]
    });
    let (status, body) = status_of(
        &client,
        "PUT",
        &draft_path,
        "uc-write",
        Some(serde_json::json!({"base_revision": revision, "document": document})),
        None,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
    let (status, _) = status_of(&client, "GET", &use_cases, "uc-read", None, None).await;
    assert_eq!(status, Status::Ok);
    let (status, _) = status_of(&client, "GET", &draft_path, "uc-read", None, None).await;
    assert_eq!(status, Status::Ok);

    // (method, path, token lacking the scope, token holding it, expected status with scope)
    let links = format!("{RES_BASE}/requirement-version-links");
    let reads: Vec<(String, &str, &str, Status)> = vec![
        (links.clone(), "req-read", "trace-read", Status::Ok),
        (
            format!("{links}/link-types"),
            "req-read",
            "trace-read",
            Status::Ok,
        ),
        (
            format!("{RES_BASE}/verifications/1/snapshots"),
            "req-read",
            "ver-read",
            Status::Ok,
        ),
        (
            format!("{RES_BASE}/reviewers"),
            "req-read",
            "projects",
            Status::Ok,
        ),
        (
            format!("{RES_BASE}/me/permissions"),
            "req-read",
            "projects",
            Status::Ok,
        ),
        (
            format!("{RES_BASE}/saved_views"),
            "projects",
            "req-read",
            Status::Ok,
        ),
        (
            "/api/home/attention".to_owned(),
            "req-read",
            "projects",
            Status::Ok,
        ),
        (
            format!("{RES_BASE}/requirements/1/impacted_tests"),
            "req-read",
            "req-ver",
            Status::Ok,
        ),
        (
            format!("{RES_BASE}/attachments?entity_type=requirement&entity_id=1"),
            "req-read",
            "req-ver",
            Status::Ok,
        ),
    ];
    for (path, without, with, expected) in reads {
        let (status, body) = status_of(&client, "GET", &path, without, None, None).await;
        assert_eq!(
            status,
            Status::Forbidden,
            "GET {path} with `{without}`: {body}"
        );
        let (status, body) = status_of(&client, "GET", &path, with, None, None).await;
        assert_eq!(status, expected, "GET {path} with `{with}`: {body}");
    }
    // `verifications:read` alone is not enough for baseline contents.
    let diff = format!("{RES_BASE}/baselines/1/verifications/1/diff/current");
    let (status, _) = status_of(&client, "GET", &diff, "ver-read", None, None).await;
    assert_eq!(status, Status::Forbidden);
    let (status, body) = status_of(&client, "GET", &diff, "ver-base", None, None).await;
    assert_ne!(status, Status::Forbidden, "{body}");
    assert_ne!(status, Status::Unauthorized, "{body}");
    // A snapshot diff passes the guard with `verifications:read` (the ids are unknown).
    let snap = format!("{RES_BASE}/verifications/1/snapshots/1/diff/2");
    let (status, _) = status_of(&client, "GET", &snap, "req-read", None, None).await;
    assert_eq!(status, Status::Forbidden);
    let (status, body) = status_of(&client, "GET", &snap, "ver-read", None, None).await;
    assert_ne!(status, Status::Forbidden, "{body}");

    // ask: the guard needs `requirements:read` (RAG is disabled in the test, so 400).
    let ask = format!("{RES_BASE}/requirements/ask");
    let question = serde_json::json!({ "query": "what?" });
    let (status, _) = status_of(
        &client,
        "POST",
        &ask,
        "projects",
        Some(question.clone()),
        None,
    )
    .await;
    assert_eq!(status, Status::Forbidden);
    let (status, body) = status_of(&client, "POST", &ask, "req-read", Some(question), None).await;
    assert_ne!(status, Status::Forbidden, "{body}");

    // impacted_tests refuses a requirement of another project.
    let (status, _) = status_of(
        &client,
        "GET",
        &format!("{RES_BASE}/requirements/3/impacted_tests"),
        "req-ver",
        None,
        None,
    )
    .await;
    assert_eq!(status, Status::NotFound);

    // Link create: scope gate, then idempotent replay.
    let link = serde_json::json!({
        "source_version_id": 1, "target_version_id": 2, "link_type": "DERIVES_FROM"
    });
    let (status, _) = status_of(
        &client,
        "POST",
        &links,
        "trace-read",
        Some(link.clone()),
        Some("k1"),
    )
    .await;
    assert_eq!(status, Status::Forbidden);
    let (status, first) = status_of(
        &client,
        "POST",
        &links,
        "trace-write",
        Some(link.clone()),
        Some("k1"),
    )
    .await;
    assert_eq!(status, Status::Ok, "{first}");
    let (status, replay) = status_of(
        &client,
        "POST",
        &links,
        "trace-write",
        Some(link.clone()),
        Some("k1"),
    )
    .await;
    assert_eq!(status, Status::Ok, "{replay}");
    let first: serde_json::Value = serde_json::from_str(&first).unwrap();
    let replay: serde_json::Value = serde_json::from_str(&replay).unwrap();
    assert_eq!(first["id"], replay["id"], "a replay returns the same link");
    let (status, list) = status_of(&client, "GET", &links, "trace-read", None, None).await;
    assert_eq!(status, Status::Ok);
    let list: serde_json::Value = serde_json::from_str(&list).unwrap();
    assert_eq!(
        list.as_array().unwrap().len(),
        1,
        "the replay created no duplicate"
    );
    // The same key with a different payload is a conflict.
    let other = serde_json::json!({
        "source_version_id": 2, "target_version_id": 1, "link_type": "RELATES_TO"
    });
    let (status, _) = status_of(
        &client,
        "POST",
        &links,
        "trace-write",
        Some(other),
        Some("k1"),
    )
    .await;
    assert_eq!(status, Status::Conflict);

    // Link delete: scope gate, and a link id of another project is a 404.
    let own = first["id"].as_i64().unwrap();
    let (status, _) = status_of(
        &client,
        "DELETE",
        &format!("{links}/{own}"),
        "trace-read",
        None,
        None,
    )
    .await;
    assert_eq!(status, Status::Forbidden);
    let (status, _) = status_of(
        &client,
        "DELETE",
        &format!("{links}/500"),
        "trace-write",
        None,
        None,
    )
    .await;
    assert_eq!(status, Status::NotFound);
    let (status, body) = status_of(
        &client,
        "DELETE",
        &format!("{links}/{own}"),
        "trace-write",
        None,
        None,
    )
    .await;
    assert_eq!(status, Status::Ok, "{body}");
}
