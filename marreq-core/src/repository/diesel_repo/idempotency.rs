use chrono::{Duration, NaiveDateTime};
use diesel::prelude::*;
use diesel::sql_types::{Integer, Jsonb, Nullable, Text, Timestamp};

use super::DieselRepo;
use crate::repository::{IdempotencyClaim, IdempotencyRepository, RepoError};

#[derive(QueryableByName)]
struct StoredClaim {
    #[diesel(sql_type = Text)]
    request_hash: String,
    #[diesel(sql_type = Nullable<Jsonb>)]
    response_json: Option<serde_json::Value>,
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
            diesel::sql_query("DELETE FROM mcp_idempotency WHERE expires_at <= $1")
                .bind::<Timestamp, _>(now).execute(conn)?;
            let inserted = diesel::sql_query("INSERT INTO mcp_idempotency (user_id, principal_key, target_key, operation, idempotency_key, request_hash, expires_at) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING")
                .bind::<Integer, _>(user).bind::<Text, _>(principal).bind::<Text, _>(target)
                .bind::<Text, _>(op).bind::<Text, _>(key).bind::<Text, _>(hash)
                .bind::<Timestamp, _>(now + Duration::days(7)).execute(conn)?;
            if inserted == 1 { return Ok(IdempotencyClaim::Acquired); }
            let stored = diesel::sql_query("SELECT request_hash, response_json FROM mcp_idempotency WHERE user_id=$1 AND principal_key=$2 AND target_key=$3 AND operation=$4 AND idempotency_key=$5")
                .bind::<Integer, _>(user).bind::<Text, _>(principal).bind::<Text, _>(target)
                .bind::<Text, _>(op).bind::<Text, _>(key)
                .get_result::<StoredClaim>(conn)?;
            if stored.request_hash != hash { Ok(IdempotencyClaim::PayloadConflict) }
            else if let Some(response) = stored.response_json { Ok(IdempotencyClaim::Replay(response)) }
            else { Ok(IdempotencyClaim::Pending) }
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
}
