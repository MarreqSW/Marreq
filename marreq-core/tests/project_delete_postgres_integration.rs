// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Project deletion against PostgreSQL (issue #349): foreign keys, the
//! baseline / locked-saved-view immutability triggers and the audit log. Set
//! `MARREQ_TEST_DATABASE_URL` to a migrated, disposable database; the test
//! truncates application data.

#![cfg(not(feature = "test-helpers"))]

use diesel::connection::SimpleConnection;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sql_types::BigInt;
use marreq_core::repository::errors::RepoError;
use marreq_core::repository::{DieselRepo, ProjectsRepository};

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = BigInt)]
    n: i64,
}

fn count(conn: &mut PgConnection, sql: &str) -> i64 {
    diesel::sql_query(sql)
        .get_result::<Count>(conn)
        .unwrap_or_else(|e| panic!("{sql}: {e}"))
        .n
}

/// One project with every kind of data that hangs off it. Ids are `base + n`
/// so the two projects never share ids.
fn seed_project(project: i32, base: i32, slug: &str) -> String {
    let b = base;
    format!(
        "INSERT INTO projects (id,name,description,status,owner_id,slug) VALUES ({project},'Project {slug}','','active',1,'{slug}');
         INSERT INTO project_members (project_id,user_id,role) VALUES ({project},1,1),({project},2,3);
         INSERT INTO project_reviewers (project_id,user_id) VALUES ({project},1);
         INSERT INTO requirement_status (id,title,description,tag,project_id) VALUES ({b},'Draft','','D{b}',{project});
         INSERT INTO verification_status (id,title,description,tag,project_id) VALUES ({b},'Passed','','P{b}',{project});
         INSERT INTO categories (id,title,description,tag,project_id) VALUES ({b},'General','','G{b}',{project});
         INSERT INTO applicability (id,title,description,tag,project_id) VALUES ({b},'All','','A{b}',{project});
         INSERT INTO verification_methods (id,title,description,tag,project_id) VALUES ({b},'Test','','T{b}',{project});
         INSERT INTO custom_field_definitions (id,project_id,label,field_type) VALUES ({b},{project},'Owner','text');
         INSERT INTO requirements (id,project_id,stable_code) VALUES ({b}+1,{project},'REQ-{b}-1'),({b}+2,{project},'REQ-{b}-2');
         INSERT INTO requirement_versions (id,requirement_id,title,description,status_id,author_id,reviewer_id,category_id,applicability_id)
           VALUES ({b}+1,{b}+1,'Parent','text',{b},1,1,{b},{b}),({b}+2,{b}+2,'Child','text',{b},1,1,{b},{b});
         UPDATE requirements SET current_version_id = id WHERE project_id = {project};
         INSERT INTO requirement_version_verification_methods (requirement_version_id,verification_method_id) VALUES ({b}+1,{b});
         INSERT INTO custom_field_values (requirement_version_id,custom_field_definition_id,value) VALUES ({b}+1,{b},'Alice');
         INSERT INTO requirement_version_links (source_version_id,target_version_id,link_type,project_id) VALUES ({b}+2,{b}+1,'DERIVES_FROM',{project});
         INSERT INTO requirement_comments (requirement_id,requirement_version_id,author_id,body) VALUES ({b}+1,{b}+1,1,'Looks good');
         INSERT INTO verifications (id,name,reference_code,description,source,status_id,project_id,verification_method_id,author_id,reviewer_id)
           VALUES ({b}+1,'Parent test','VER-{b}-1','','',{b},{project},{b},1,1);
         INSERT INTO verifications (id,name,reference_code,description,source,status_id,parent_id,project_id,verification_method_id,author_id,reviewer_id)
           VALUES ({b}+2,'Child test','VER-{b}-2','','',{b},{b}+1,{project},{b},1,1);
         INSERT INTO matrix (req_id,verification_id,project_id,triggering_version_id) VALUES ({b}+1,{b}+1,{project},{b}+1);
         INSERT INTO attachments (id,project_id,entity_type,entity_id,sha256,size_bytes,original_filename,content_type)
           VALUES ({b},{project},'requirement',{b}+1,repeat('a',63)||'{}',10,'spec.pdf','application/pdf');
         INSERT INTO project_storage_quotas (project_id,quota_bytes) VALUES ({project},1048576);
         INSERT INTO saved_views (id,project_id,owner_id,name,definition,visibility,locked) VALUES ({b},{project},1,'CDR scope','{{}}','shared',true);
         INSERT INTO baselines (id,project_id,name,created_by,source_saved_view_id) VALUES ({b},{project},'CDR',1,{b});
         INSERT INTO baseline_requirements (baseline_id,requirement_id,version_id) VALUES ({b},{b}+1,{b}+1);
         INSERT INTO baseline_traceability (baseline_id,requirement_id,verification_id) VALUES ({b},{b}+1,{b}+1);
         INSERT INTO baseline_verifications (baseline_id,verification_id,name,reference_code,status_id,project_id,author_id,reviewer_id)
           VALUES ({b},{b}+1,'Parent test','VER-{b}-1',{b},{project},1,1);
         INSERT INTO baseline_attachments (baseline_id,attachment_id) VALUES ({b},{b});
         INSERT INTO verification_control (verification_id,project_id,verification_level,evidence_reference) VALUES ({b}+1,{project},'System','TR-{b}');
         INSERT INTO requirement_compliance (requirement_id,project_id,compliance,note) VALUES ({b}+1,{project},'C','accepted');
         INSERT INTO notifications (user_id,project_id,notification_type,title) VALUES (2,{project},'requirement_updated','Changed');
         INSERT INTO notification_preferences (user_id,project_id) VALUES (2,{project});
         INSERT INTO user_api_tokens (user_id,token_hash,project_id) VALUES (2,'token-{b}',{project});
         INSERT INTO logs (user_id,action_type,entity_type,entity_id,project_id,description) VALUES (1,'CREATE','PROJECT',{project},{project},'created');",
        project % 10
    )
}

/// Rows in every table that belongs to project `p` (by `project_id` or through
/// its requirements / baselines).
fn project_rows(conn: &mut PgConnection, p: i32) -> Vec<(&'static str, i64)> {
    let by_project = [
        "project_members",
        "project_reviewers",
        "requirement_status",
        "verification_status",
        "categories",
        "applicability",
        "verification_methods",
        "custom_field_definitions",
        "requirements",
        "requirement_version_links",
        "verifications",
        "matrix",
        "attachments",
        "project_storage_quotas",
        "saved_views",
        "baselines",
        "baseline_verifications",
        "notifications",
        "notification_preferences",
        "user_api_tokens",
        "verification_control",
        "requirement_compliance",
    ];
    let mut out: Vec<(&'static str, i64)> = by_project
        .iter()
        .map(|t| {
            (
                *t,
                count(
                    conn,
                    &format!("SELECT count(*) AS n FROM {t} WHERE project_id = {p}"),
                ),
            )
        })
        .collect();
    let lo = p * 100;
    let hi = lo + 99;
    for (t, col) in [
        ("requirement_versions", "id"),
        (
            "requirement_version_verification_methods",
            "requirement_version_id",
        ),
        ("custom_field_values", "requirement_version_id"),
        ("requirement_comments", "requirement_id"),
        ("baseline_requirements", "baseline_id"),
        ("baseline_traceability", "baseline_id"),
        ("baseline_attachments", "baseline_id"),
    ] {
        out.push((
            t,
            count(
                conn,
                &format!("SELECT count(*) AS n FROM {t} WHERE {col} BETWEEN {lo} AND {hi}"),
            ),
        ));
    }
    out
}

#[test]
fn deleting_a_project_removes_everything_and_keeps_baselines_immutable_elsewhere() {
    let Ok(database_url) = std::env::var("MARREQ_TEST_DATABASE_URL") else {
        eprintln!(
            "skipping PostgreSQL project deletion test: set MARREQ_TEST_DATABASE_URL to a disposable migrated database to run it"
        );
        return;
    };
    // SAFETY: this is the only test in its binary and it runs before any
    // database pool or other thread that reads the environment exists.
    unsafe {
        std::env::set_var("DATABASE_URL", &database_url);
    }

    let mut conn = PgConnection::establish(&database_url).expect("test PostgreSQL connection");
    conn.batch_execute(&format!(
        "TRUNCATE users, projects, mcp_idempotency RESTART IDENTITY CASCADE;
         INSERT INTO users (id,username,name,email,password_hash,is_admin) VALUES
           (1,'owner','Owner','owner@example.test','!',false),
           (2,'member','Member','member@example.test','!',false);
         {}
         {}",
        seed_project(1, 100, "project-a"),
        seed_project(2, 200, "project-b"),
    ))
    .expect("seed");
    let before_b = project_rows(&mut conn, 2);
    assert!(
        before_b.iter().all(|(_, n)| *n > 0),
        "seed covers every table: {before_b:?}"
    );

    marreq_core::repository::diesel_repo::init_connection_pool().expect("pool");
    let mut repo = DieselRepo::new().expect("repo");

    let purge = repo.delete_project(1).expect("project A is deleted");
    assert_eq!(purge.project.slug, "project-a");
    assert_eq!(
        (
            purge.requirements,
            purge.verifications,
            purge.baselines,
            purge.attachments
        ),
        (2, 2, 1, 1)
    );
    assert_eq!(purge.attachment_blobs.len(), 1);
    let mut members = purge.member_ids.clone();
    members.sort();
    assert_eq!(members, vec![1, 2]);

    let left = project_rows(&mut conn, 1);
    assert!(
        left.iter().all(|(_, n)| *n == 0),
        "rows of project A left behind: {left:?}"
    );
    assert_eq!(
        count(&mut conn, "SELECT count(*) AS n FROM projects WHERE id = 1"),
        0
    );
    assert_eq!(
        count(
            &mut conn,
            "SELECT count(*) AS n FROM logs WHERE project_id IS NULL AND entity_id = 1"
        ),
        1,
        "project A's audit history is kept, detached from the project"
    );
    assert_eq!(
        project_rows(&mut conn, 2),
        before_b,
        "project B is untouched"
    );
    assert_eq!(
        count(&mut conn, "SELECT count(*) AS n FROM users"),
        2,
        "users are kept"
    );

    // The trigger bypass was local to the purge transaction: on the pool's
    // connections and on a fresh one, baselines and locked views stay immutable.
    for sql in [
        "DELETE FROM baselines WHERE project_id = 2",
        "DELETE FROM baseline_requirements WHERE baseline_id = 200",
        "DELETE FROM saved_views WHERE project_id = 2",
        "UPDATE baselines SET name = 'x' WHERE project_id = 2",
    ] {
        let mut pooled = repo.get_conn().expect("pooled connection");
        let err = diesel::sql_query(sql)
            .execute(pooled.as_mut())
            .expect_err(sql);
        assert!(err.to_string().contains("immutable"), "{sql}: {err}");
        let err = diesel::sql_query(sql).execute(&mut conn).expect_err(sql);
        assert!(err.to_string().contains("immutable"), "{sql}: {err}");
    }

    assert!(matches!(repo.delete_project(1), Err(RepoError::NotFound)));

    // Project B can be deleted too, leaving no project data at all.
    repo.delete_project(2).expect("project B is deleted");
    assert!(project_rows(&mut conn, 2).iter().all(|(_, n)| *n == 0));
    assert_eq!(
        count(
            &mut conn,
            "SELECT count(*) AS n FROM logs WHERE project_id IS NULL"
        ),
        2
    );

    wait_for_quiet_pool(&repo);
}

/// r2d2 opens connections in the background to keep `min_idle` after
/// checkouts. Exiting while one is still being established can crash in
/// libpq / OpenSSL teardown (seen in CI as SIGSEGV after the test passed), so
/// wait until every connection the pool counts is established and idle.
fn wait_for_quiet_pool(repo: &DieselRepo) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while std::time::Instant::now() < deadline {
        let s = repo.pool_stats();
        if s.available == s.current_size && s.available >= s.min_idle {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}
