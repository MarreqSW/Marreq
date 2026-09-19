use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request, State};
use sha2::{Digest, Sha256};

use crate::api::prelude::*;
use crate::repository::{IdempotencyClaim, IdempotencyRepository};

pub struct OptionalIdempotencyKey(pub Option<String>);

#[async_trait]
impl<'r> FromRequest<'r> for OptionalIdempotencyKey {
    type Error = ();
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let key = request.headers().get_one("Idempotency-Key").map(str::trim);
        match key {
            Some(value)
                if value.is_empty()
                    || value.len() > 200
                    || !value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.')) =>
            {
                Outcome::Error((Status::BadRequest, ()))
            }
            value => Outcome::Success(Self(value.map(str::to_owned))),
        }
    }
}

pub fn claim(
    state: &State<AppState>,
    user_id: i32,
    operation: &str,
    key: &OptionalIdempotencyKey,
    payload: &impl serde::Serialize,
) -> ApiResult<Option<serde_json::Value>> {
    let Some(key) = key.0.as_deref() else {
        return Ok(None);
    };
    let encoded =
        serde_json::to_vec(payload).map_err(|_| ApiError::BadRequest("invalid payload".into()))?;
    let hash = format!("{:x}", Sha256::digest(encoded));
    match state.repo_write().claim_idempotency(
        user_id,
        operation,
        key,
        &hash,
        chrono::Utc::now().naive_utc(),
    )? {
        IdempotencyClaim::Acquired => Ok(None),
        IdempotencyClaim::Replay(response) => Ok(Some(response)),
        IdempotencyClaim::Pending => Err(ApiError::Conflict(
            "operation with this idempotency key is still pending".into(),
        )),
        IdempotencyClaim::PayloadConflict => Err(ApiError::Conflict(
            "idempotency key was already used with a different payload".into(),
        )),
    }
}

pub fn complete(
    state: &State<AppState>,
    user_id: i32,
    operation: &str,
    key: &OptionalIdempotencyKey,
    response: &serde_json::Value,
) -> ApiResult<()> {
    if let Some(key) = key.0.as_deref() {
        state
            .repo_write()
            .complete_idempotency(user_id, operation, key, response)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;

    #[test]
    fn same_key_replays_and_different_payload_conflicts() {
        let mut repo = DieselRepoMock::default();
        let now = chrono::Utc::now().naive_utc();
        assert_eq!(
            repo.claim_idempotency(1, "create_requirement", "key", "hash-a", now)
                .unwrap(),
            IdempotencyClaim::Acquired
        );
        assert_eq!(
            repo.claim_idempotency(1, "create_requirement", "key", "hash-a", now)
                .unwrap(),
            IdempotencyClaim::Pending
        );
        let response = serde_json::json!({"id": 42});
        repo.complete_idempotency(1, "create_requirement", "key", &response)
            .unwrap();
        assert_eq!(
            repo.claim_idempotency(1, "create_requirement", "key", "hash-a", now)
                .unwrap(),
            IdempotencyClaim::Replay(response)
        );
        assert_eq!(
            repo.claim_idempotency(1, "create_requirement", "key", "hash-b", now)
                .unwrap(),
            IdempotencyClaim::PayloadConflict
        );
    }
}
