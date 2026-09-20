// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Production-path delegated OAuth tests. Set `MARREQ_TEST_DATABASE_URL` to a
//! migrated, disposable PostgreSQL database; the test truncates application data.

#![cfg(not(feature = "test-helpers"))]

use chrono::{Duration, Utc};
use diesel::connection::SimpleConnection;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sql_types::{Array, Integer, Text};
use marreq_core::auth::delegated::hash_secret;
use marreq_core::repository::{DieselRepo, IdempotencyClaim, IdempotencyRepository};
use rocket::http::{ContentType, Header, Status};
use rocket::local::asynchronous::{Client, LocalResponse};

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

fn challenge(response: &LocalResponse<'_>) -> Option<String> {
    response
        .headers()
        .get_one("WWW-Authenticate")
        .map(str::to_owned)
}

#[rocket::async_test]
async fn delegated_scope_rbac_revocation_and_downgrade_matrix() {
    let Ok(database_url) = std::env::var("MARREQ_TEST_DATABASE_URL") else {
        eprintln!("skipping PostgreSQL OAuth integration test: MARREQ_TEST_DATABASE_URL is unset");
        return;
    };
    std::env::set_var("DATABASE_URL", &database_url);
    std::env::set_var("MARREQ_PUBLIC_BASE_URL", "http://localhost:8080");
    std::env::set_var("MARREQ_MCP_PUBLIC_URL", "http://localhost:8080/mcp/");
    std::env::set_var(
        "MARREQ_MCP_AUDIT_SECRET",
        "integration-test-audit-secret-32-bytes",
    );

    let mut conn = PgConnection::establish(&database_url).expect("test PostgreSQL connection");
    conn.batch_execute(
        "TRUNCATE users, projects, oauth_clients RESTART IDENTITY CASCADE;
         INSERT INTO users (id,username,name,email,password_hash,is_admin) VALUES
           (1,'reviewer','Reviewer','reviewer@example.test','!',false),
           (2,'viewer','Viewer','viewer@example.test','!',false),
           (3,'author','Author','author@example.test','!',false),
           (4,'outsider','Outsider','outsider@example.test','!',false);
         INSERT INTO projects (id,name,description,status,owner_id,slug) VALUES
           (1,'Project A','A','active',1,'project-a'),
           (2,'Project B','B','active',NULL,'project-b');
         INSERT INTO project_members (project_id,user_id,role) VALUES (1,1,2),(1,2,4),(1,3,3);
         INSERT INTO project_reviewers (project_id,user_id) VALUES (1,1);
         INSERT INTO requirement_status (id,title,description,tag,project_id,is_system) VALUES (1,'Draft','','DRAFT',1,true);
         INSERT INTO categories (id,title,description,tag,project_id) VALUES (1,'General','','GEN',1);
         INSERT INTO applicability (id,title,description,tag,project_id) VALUES (1,'All','','ALL',1);
         INSERT INTO verification_methods (id,title,description,tag,project_id) VALUES (1,'Analysis','','AN',1);
         INSERT INTO verification_status (id,title,description,tag,project_id,is_system) VALUES (1,'Not run','','NOT_RUN',1,true),(2,'Passed','','PASSED',1,true);
         INSERT INTO requirements (id,project_id,stable_code) VALUES (1,1,'REQ-1');
         INSERT INTO requirement_versions (id,requirement_id,title,description,status_id,author_id,reviewer_id,category_id,applicability_id,approval_state)
           VALUES (1,1,'Requirement','Description',1,3,1,1,1,'draft');
         UPDATE requirements SET current_version_id=1 WHERE id=1;
         INSERT INTO verifications (id,name,reference_code,description,source,status_id,project_id,verification_method_id,author_id,reviewer_id)
           VALUES (1,'Verification','VER-1','Description','manual',1,1,1,3,1);",
    ).expect("seed OAuth integration fixtures");

    let projects = insert_token(&mut conn, "projects", 1, &["projects:read"], false, false);
    let req_read = insert_token(
        &mut conn,
        "req-read",
        1,
        &["requirements:read"],
        false,
        false,
    );
    insert_token(
        &mut conn,
        "req-write",
        1,
        &["requirements:write"],
        false,
        false,
    );
    insert_token(
        &mut conn,
        "viewer-write",
        2,
        &["requirements:write"],
        false,
        false,
    );
    insert_token(
        &mut conn,
        "reviewer-approve",
        1,
        &["requirements:approve"],
        false,
        false,
    );
    insert_token(
        &mut conn,
        "author-approve",
        3,
        &["requirements:approve"],
        false,
        false,
    );
    insert_token(
        &mut conn,
        "outsider-read",
        4,
        &["requirements:read"],
        false,
        false,
    );
    insert_token(
        &mut conn,
        "verification-read",
        1,
        &["verifications:read"],
        false,
        false,
    );
    insert_token(
        &mut conn,
        "verification-write",
        1,
        &["verifications:write"],
        false,
        false,
    );
    insert_token(
        &mut conn,
        "author-verification-write",
        3,
        &["verifications:write"],
        false,
        false,
    );
    insert_token(
        &mut conn,
        "trace-up",
        1,
        &["requirements:read", "traceability:read"],
        false,
        false,
    );
    insert_token(
        &mut conn,
        "trace-down",
        1,
        &[
            "requirements:read",
            "verifications:read",
            "traceability:read",
        ],
        false,
        false,
    );
    insert_token(
        &mut conn,
        "baseline",
        1,
        &["requirements:read", "baselines:read"],
        false,
        false,
    );
    insert_token(
        &mut conn,
        "principal-only",
        1,
        &["requirements:read"],
        false,
        false,
    );
    insert_token(&mut conn, "expired", 1, &["requirements:read"], true, false);
    insert_token(&mut conn, "revoked", 1, &["requirements:read"], false, true);
    let downgrade_grant = insert_token(
        &mut conn,
        "downgrade",
        1,
        &["requirements:read", "requirements:write"],
        false,
        false,
    );
    let revocable_grant = insert_token(
        &mut conn,
        "revocable",
        1,
        &["requirements:read"],
        false,
        false,
    );

    marreq_core::config::AppConfig::install_from_env_or_exit();
    let rocket = marreq_core::app::build_with_auth(
        &TEST_MODE,
        marreq_core::auth::AuthConfig::default(),
        vec![],
        vec![],
    );
    let client = Client::tracked(rocket).await.expect("Rocket client");

    let response = client
        .get("/api/projects")
        .header(bearer("projects"))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let response = client
        .get("/api/projects")
        .header(bearer("req-read"))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
    assert!(challenge(&response)
        .is_some_and(|v| v.contains("insufficient_scope") && v.contains("projects:read")));

    let response = client
        .get("/api/projects/1/requirements")
        .header(bearer("req-read"))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let response = client
        .get("/api/projects/1/requirements")
        .header(bearer("projects"))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
    assert!(challenge(&response).is_some_and(|v| v.contains("insufficient_scope")));
    let response = client
        .get("/api/projects/1/requirements")
        .header(bearer("outsider-read"))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
    assert!(
        challenge(&response).is_none(),
        "RBAC denial must not request OAuth reconnect"
    );
    let response = client
        .get("/api/projects/2/requirements")
        .header(bearer("req-read"))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
    assert!(challenge(&response).is_none());

    let patch = serde_json::json!({"title":"Changed"}).to_string();
    let response = client
        .patch("/api/projects/1/requirements/1")
        .header(ContentType::JSON)
        .header(bearer("req-write"))
        .body(&patch)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let response = client
        .patch("/api/projects/1/requirements/1")
        .header(ContentType::JSON)
        .header(bearer("req-read"))
        .body(&patch)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
    assert!(challenge(&response).is_some_and(|v| v.contains("requirements:write")));
    let response = client
        .patch("/api/projects/1/requirements/1")
        .header(ContentType::JSON)
        .header(bearer("viewer-write"))
        .body(&patch)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
    assert!(challenge(&response).is_none());

    let approval = serde_json::json!({"state":"reviewed"}).to_string();
    let response = client
        .put("/api/projects/1/requirements/1/versions/1/approval")
        .header(ContentType::JSON)
        .header(bearer("author-approve"))
        .body(&approval)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
    assert!(challenge(&response).is_none());
    let response = client
        .put("/api/projects/1/requirements/1/versions/1/approval")
        .header(ContentType::JSON)
        .header(bearer("req-read"))
        .body(&approval)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
    assert!(challenge(&response).is_some_and(|v| v.contains("requirements:approve")));
    let response = client
        .put("/api/projects/1/requirements/1/versions/1/approval")
        .header(ContentType::JSON)
        .header(bearer("reviewer-approve"))
        .body(&approval)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);

    assert_eq!(
        client
            .get("/api/projects/1/verifications")
            .header(bearer("verification-read"))
            .dispatch()
            .await
            .status(),
        Status::Ok
    );
    let response = client
        .get("/api/projects/1/verifications")
        .header(bearer("req-read"))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
    assert!(challenge(&response).is_some_and(|v| v.contains("verifications:read")));
    let status_update = serde_json::json!({"field":"status_id","value":"2"}).to_string();
    let response = client
        .post("/api/projects/1/verifications/1/field")
        .header(ContentType::JSON)
        .header(bearer("author-verification-write"))
        .body(&status_update)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
    assert!(challenge(&response).is_none());
    assert_eq!(
        client
            .post("/api/projects/1/verifications/1/field")
            .header(ContentType::JSON)
            .header(bearer("verification-write"))
            .body(&status_update)
            .dispatch()
            .await
            .status(),
        Status::Ok
    );

    assert_eq!(
        client
            .get("/api/projects/1/requirements/1/trace_up")
            .header(bearer("trace-up"))
            .dispatch()
            .await
            .status(),
        Status::Ok
    );
    for token in ["req-read", "projects"] {
        let response = client
            .get("/api/projects/1/requirements/1/trace_up")
            .header(bearer(token))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
        assert!(challenge(&response).is_some_and(|v| v.contains("insufficient_scope")));
    }
    assert_eq!(
        client
            .get("/api/projects/1/requirements/1/trace_down")
            .header(bearer("trace-down"))
            .dispatch()
            .await
            .status(),
        Status::Ok
    );
    for token in ["trace-up", "verification-read", "req-read"] {
        let response = client
            .get("/api/projects/1/requirements/1/trace_down")
            .header(bearer(token))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
        assert!(challenge(&response).is_some_and(|v| v.contains("insufficient_scope")));
    }
    assert_eq!(
        client
            .get("/api/projects/1/baselines/999/requirements/1/diff/current")
            .header(bearer("baseline"))
            .dispatch()
            .await
            .status(),
        Status::NotFound
    );
    for token in ["req-read", "projects"] {
        let response = client
            .get("/api/projects/1/baselines/999/requirements/1/diff/current")
            .header(bearer(token))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
        assert!(challenge(&response).is_some_and(|v| v.contains("insufficient_scope")));
    }

    assert_eq!(
        client
            .get("/api/mcp/principal")
            .header(bearer("principal-only"))
            .dispatch()
            .await
            .status(),
        Status::Ok
    );
    for token in ["expired", "revoked"] {
        let response = client
            .get("/api/mcp/principal")
            .header(bearer(token))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Unauthorized);
        assert!(challenge(&response).is_some_and(|v| v.contains("invalid_token")));
    }

    assert_eq!(
        client
            .get("/api/projects/1/requirements")
            .header(bearer("revocable"))
            .dispatch()
            .await
            .status(),
        Status::Ok
    );
    diesel::sql_query("UPDATE oauth_grants SET revoked_at=NOW() WHERE id=$1")
        .bind::<Integer, _>(revocable_grant)
        .execute(&mut conn)
        .unwrap();
    let response = client
        .get("/api/projects/1/requirements")
        .header(bearer("revocable"))
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Unauthorized);
    assert!(challenge(&response).is_some_and(|v| v.contains("invalid_token")));

    assert_eq!(
        client
            .patch("/api/projects/1/requirements/1")
            .header(ContentType::JSON)
            .header(bearer("downgrade"))
            .body(&patch)
            .dispatch()
            .await
            .status(),
        Status::Ok
    );
    diesel::sql_query(
        "UPDATE oauth_grants SET scopes=ARRAY['requirements:read']::text[] WHERE id=$1",
    )
    .bind::<Integer, _>(downgrade_grant)
    .execute(&mut conn)
    .unwrap();
    let response = client
        .patch("/api/projects/1/requirements/1")
        .header(ContentType::JSON)
        .header(bearer("downgrade"))
        .body(&patch)
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Forbidden);
    assert!(challenge(&response).is_some_and(|v| v.contains("insufficient_scope")));

    // PostgreSQL idempotency lifecycle: payload conflicts, release/reuse,
    // stale-lease takeover, crash recovery from every create-like domain row,
    // and concurrent single ownership.
    let mut repo = DieselRepo::new().expect("production repository");
    let now = Utc::now().naive_utc();
    assert_eq!(
        repo.claim_idempotency(1, "oauth_grant:1", "project:1", "test-op", "same", "a", now)
            .unwrap(),
        IdempotencyClaim::Acquired
    );
    assert_eq!(
        repo.claim_idempotency(1, "oauth_grant:1", "project:1", "test-op", "same", "b", now)
            .unwrap(),
        IdempotencyClaim::PayloadConflict
    );
    repo.release_idempotency(1, "oauth_grant:1", "project:1", "test-op", "same")
        .unwrap();
    assert_eq!(
        repo.claim_idempotency(1, "oauth_grant:1", "project:1", "test-op", "same", "b", now)
            .unwrap(),
        IdempotencyClaim::Acquired,
        "known failure releases the key for reuse"
    );
    assert_eq!(
        repo.claim_idempotency(
            1,
            "oauth_grant:1",
            "project:1",
            "stale-op",
            "stale",
            "a",
            now - Duration::minutes(3),
        )
        .unwrap(),
        IdempotencyClaim::Acquired
    );
    assert_eq!(
        repo.claim_idempotency(
            1,
            "oauth_grant:1",
            "project:1",
            "stale-op",
            "stale",
            "a",
            now,
        )
        .unwrap(),
        IdempotencyClaim::Acquired,
        "expired pending lease is recoverable"
    );

    fn operation_identity(operation: &str, target: &str, key: &str) -> String {
        use sha2::{Digest, Sha256};
        let value = format!(
            "{}\0{}\0{}\0{}\0{}",
            1, "oauth_grant:1", target, operation, key
        );
        format!("{:x}", Sha256::digest(value.as_bytes()))
    }
    for (operation, target, key, insert_sql) in [
        (
            "create_requirement",
            "project:1",
            "recover-requirement",
            "UPDATE requirements SET mcp_idempotency_identity=$1 WHERE id=1",
        ),
        (
            "create_verification",
            "project:1",
            "recover-verification",
            "UPDATE verifications SET mcp_idempotency_identity=$1 WHERE id=1",
        ),
        (
            "create_baseline",
            "project:1",
            "recover-baseline",
            "INSERT INTO baselines (project_id,name,created_by,mcp_idempotency_identity) VALUES (1,'Recovered',1,$1)",
        ),
        (
            "create_requirement_comment",
            "project:1:requirement:1",
            "recover-comment",
            "INSERT INTO requirement_comments (requirement_id,author_id,body,mcp_idempotency_identity) VALUES (1,1,'Recovered',$1)",
        ),
    ] {
        assert_eq!(
            repo.claim_idempotency(1, "oauth_grant:1", target, operation, key, "payload", now)
                .unwrap(),
            IdempotencyClaim::Acquired
        );
        diesel::sql_query(insert_sql)
            .bind::<Text, _>(operation_identity(operation, target, key))
            .execute(&mut conn)
            .unwrap();
        assert!(matches!(
            repo.claim_idempotency(1, "oauth_grant:1", target, operation, key, "payload", now),
            Ok(IdempotencyClaim::Replay(_))
        ));
    }

    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let barrier = barrier.clone();
        handles.push(std::thread::spawn(move || {
            let mut repo = DieselRepo::new().unwrap();
            barrier.wait();
            repo.claim_idempotency(
                1,
                "oauth_grant:1",
                "project:1",
                "concurrent-op",
                "concurrent",
                "payload",
                Utc::now().naive_utc(),
            )
            .unwrap()
        }));
    }
    barrier.wait();
    let claims = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        claims
            .iter()
            .filter(|claim| **claim == IdempotencyClaim::Acquired)
            .count(),
        1,
        "two concurrent claims have exactly one owner"
    );

    // Keep otherwise-unused fixture ids asserted so accidental fixture creation failures are visible.
    assert!(projects > 0 && req_read > 0);
}
