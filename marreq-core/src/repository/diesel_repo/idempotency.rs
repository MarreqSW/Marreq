use chrono::{Duration, NaiveDateTime};
use diesel::prelude::*;
use diesel::sql_types::{Integer, Jsonb, Nullable, Text, Timestamp};
use sha2::{Digest, Sha256};

use super::DieselRepo;
use crate::repository::{IdempotencyClaim, IdempotencyRepository, RepoError};

#[derive(QueryableByName)]
struct StoredClaim {
    #[diesel(sql_type = Text)]
    request_hash: String,
    #[diesel(sql_type = Nullable<Jsonb>)]
    response_json: Option<serde_json::Value>,
    #[diesel(sql_type = Timestamp)]
    lease_expires_at: NaiveDateTime,
}

#[derive(QueryableByName)]
struct RecoveredResponse {
    #[diesel(sql_type = Jsonb)]
    response_json: serde_json::Value,
}

fn identity(user: i32, principal: &str, target: &str, op: &str, key: &str) -> String {
    let value = format!("{user}\0{principal}\0{target}\0{op}\0{key}");
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn recover_response(
    conn: &mut PgConnection,
    operation: &str,
    operation_identity: &str,
) -> QueryResult<Option<serde_json::Value>> {
    let query = match operation {
        "create_requirement" => "SELECT jsonb_build_object('status','ok','id',id) AS response_json FROM requirements WHERE mcp_idempotency_identity=$1",
        "create_verification" => "SELECT jsonb_build_object('status','ok','id',id) AS response_json FROM verifications WHERE mcp_idempotency_identity=$1",
        "create_baseline" => "SELECT to_jsonb(b) - 'mcp_idempotency_identity' AS response_json FROM baselines b WHERE mcp_idempotency_identity=$1",
        "create_requirement_comment" => "SELECT jsonb_build_object('id',c.id,'requirement_id',c.requirement_id,'requirement_version_id',c.requirement_version_id,'author_id',c.author_id,'author_name',COALESCE(u.name,'User#' || c.author_id::text),'body',c.body,'created_at',c.created_at) AS response_json FROM requirement_comments c LEFT JOIN users u ON u.id=c.author_id WHERE c.mcp_idempotency_identity=$1",
        _ => return Ok(None),
    };
    diesel::sql_query(query)
        .bind::<Text, _>(operation_identity)
        .get_result::<RecoveredResponse>(conn)
        .optional()
        .map(|value| value.map(|row| row.response_json))
}

impl IdempotencyRepository for DieselRepo {
    fn claim_idempotency(
        &mut self,
        user: i32,
        principal: &str,
        target: &str,
        op: &str,
        key: &str,
        hash: &str,
        now: NaiveDateTime,
    ) -> Result<IdempotencyClaim, RepoError> {
        let mut conn = self.get_conn()?;
        conn.transaction::<IdempotencyClaim, diesel::result::Error, _>(|conn| {
            let operation_identity = identity(user, principal, target, op, key);
            let inserted = diesel::sql_query("INSERT INTO mcp_idempotency (user_id, principal_key, target_key, operation, idempotency_key, request_hash, lease_expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING")
                .bind::<Integer, _>(user).bind::<Text, _>(principal).bind::<Text, _>(target)
                .bind::<Text, _>(op).bind::<Text, _>(key).bind::<Text, _>(hash)
                .bind::<Timestamp, _>(now + Duration::minutes(2))
                .execute(conn)?;
            if inserted == 1 {
                // A domain row can outlive its claim only after an interrupted older
                // deployment or manual repair. Recover it here as well so retrying can
                // never attempt a second create merely because the claim row was absent.
                if let Some(response) = recover_response(conn, op, &operation_identity)? {
                    diesel::sql_query("UPDATE mcp_idempotency SET response_json=$6 WHERE user_id=$1 AND principal_key=$2 AND target_key=$3 AND operation=$4 AND idempotency_key=$5")
                        .bind::<Integer, _>(user).bind::<Text, _>(principal).bind::<Text, _>(target)
                        .bind::<Text, _>(op).bind::<Text, _>(key).bind::<Jsonb, _>(&response).execute(conn)?;
                    return Ok(IdempotencyClaim::Replay(response));
                }
                return Ok(IdempotencyClaim::Acquired);
            }
            let stored = diesel::sql_query("SELECT request_hash, response_json, lease_expires_at FROM mcp_idempotency WHERE user_id=$1 AND principal_key=$2 AND target_key=$3 AND operation=$4 AND idempotency_key=$5 FOR UPDATE")
                .bind::<Integer, _>(user).bind::<Text, _>(principal).bind::<Text, _>(target)
                .bind::<Text, _>(op).bind::<Text, _>(key)
                .get_result::<StoredClaim>(conn)?;
            if stored.request_hash != hash { Ok(IdempotencyClaim::PayloadConflict) }
            else if let Some(response) = stored.response_json { Ok(IdempotencyClaim::Replay(response)) }
            else if let Some(response) = recover_response(conn, op, &operation_identity)? {
                diesel::sql_query("UPDATE mcp_idempotency SET response_json=$6 WHERE user_id=$1 AND principal_key=$2 AND target_key=$3 AND operation=$4 AND idempotency_key=$5")
                    .bind::<Integer, _>(user).bind::<Text, _>(principal).bind::<Text, _>(target)
                    .bind::<Text, _>(op).bind::<Text, _>(key).bind::<Jsonb, _>(&response).execute(conn)?;
                Ok(IdempotencyClaim::Replay(response))
            } else if stored.lease_expires_at <= now {
                diesel::sql_query("UPDATE mcp_idempotency SET lease_expires_at=$6 WHERE user_id=$1 AND principal_key=$2 AND target_key=$3 AND operation=$4 AND idempotency_key=$5")
                    .bind::<Integer, _>(user).bind::<Text, _>(principal).bind::<Text, _>(target)
                    .bind::<Text, _>(op).bind::<Text, _>(key)
                    .bind::<Timestamp, _>(now + Duration::minutes(2))
                    .execute(conn)?;
                Ok(IdempotencyClaim::Acquired)
            } else { Ok(IdempotencyClaim::Pending) }
        }).map_err(Into::into)
    }

    fn complete_idempotency(
        &mut self,
        user: i32,
        principal: &str,
        target: &str,
        op: &str,
        key: &str,
        response: &serde_json::Value,
    ) -> Result<(), RepoError> {
        diesel::sql_query("UPDATE mcp_idempotency SET response_json=$6 WHERE user_id=$1 AND principal_key=$2 AND target_key=$3 AND operation=$4 AND idempotency_key=$5")
            .bind::<Integer, _>(user).bind::<Text, _>(principal).bind::<Text, _>(target)
            .bind::<Text, _>(op).bind::<Text, _>(key).bind::<Jsonb, _>(response)
            .execute(&mut *self.get_conn()?).map(|_| ()).map_err(Into::into)
    }

    fn release_idempotency(
        &mut self,
        user: i32,
        principal: &str,
        target: &str,
        op: &str,
        key: &str,
    ) -> Result<(), RepoError> {
        let mut conn = self.get_conn()?;
        conn.transaction::<(), diesel::result::Error, _>(|conn| {
            let operation_identity = identity(user, principal, target, op, key);
            if let Some(response) = recover_response(conn, op, &operation_identity)? {
                // The domain transaction committed. Preserve and complete the claim;
                // deleting it here would turn a concurrent retry into an ambiguous write.
                diesel::sql_query("UPDATE mcp_idempotency SET response_json=$6 WHERE user_id=$1 AND principal_key=$2 AND target_key=$3 AND operation=$4 AND idempotency_key=$5")
                    .bind::<Integer, _>(user).bind::<Text, _>(principal).bind::<Text, _>(target)
                    .bind::<Text, _>(op).bind::<Text, _>(key).bind::<Jsonb, _>(&response).execute(conn)?;
            } else {
                diesel::sql_query("DELETE FROM mcp_idempotency WHERE user_id=$1 AND principal_key=$2 AND target_key=$3 AND operation=$4 AND idempotency_key=$5 AND response_json IS NULL")
                    .bind::<Integer, _>(user).bind::<Text, _>(principal).bind::<Text, _>(target)
                    .bind::<Text, _>(op).bind::<Text, _>(key).execute(conn)?;
            }
            Ok(())
        }).map_err(Into::into)
    }
}
