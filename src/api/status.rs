use crate::api::prelude::*;
use crate::models::{NewStatus, RequirementStatus, Status as LegacyStatus};
use crate::services::StatusService;

#[get("/status")]
pub async fn list(state: &State<AppState>) -> ApiResult<Json<Vec<LegacyStatus>>> {
    let service = StatusService::new(state.inner());
    let statuses = service
        .list_requirement_statuses()?
        .into_iter()
        .map(|status: RequirementStatus| LegacyStatus {
            st_id: status.req_st_id,
            st_title: status.req_st_title,
            st_description: status.req_st_description,
            st_short_name: status.req_st_short_name,
        })
        .collect();
    Ok(Json(statuses))
}

#[get("/status/<id>")]
pub async fn get(id: i32, state: &State<AppState>) -> ApiResult<Json<Value>> {
    let service = StatusService::new(state.inner());
    let status = service.get_requirement_status(id)?;

    Ok(Json(json!({
        "id": status.req_st_id,
        "title": status.req_st_title,
        "description": status.req_st_description,
        "short_name": status.req_st_short_name,
    })))
}

#[post("/status", data = "<payload>")]
pub async fn create(
    state: &State<AppState>,
    payload: Json<NewStatus>,
) -> ApiResult<(Status, Value)> {
    let service = StatusService::new(state.inner());
    let id = service.create_requirement_status(payload.into_inner())?;

    Ok((Status::Created, json!({ "status": "ok", "id": id })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::ContentType;
    use rocket::local::asynchronous::Client;
    use serde_json::{json, Value};
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    fn state_from_repo(repo: DieselRepoMock) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(state_from_repo(repo))
            .mount("/api", routes![list, get, create]);
        Client::tracked(rocket).await.unwrap()
    }

    #[rocket::async_test]
    async fn list_returns_seeded_statuses() {
        let mut repo = DieselRepoMock::default();
        let mut statuses = HashMap::new();
        statuses.insert(
            1,
            RequirementStatus {
                req_st_id: 1,
                req_st_title: "Draft".into(),
                req_st_description: "Initial".into(),
                req_st_short_name: "DR".into(),
            },
        );
        repo.requirement_statuses = statuses;

        let client = client_with_repo(repo).await;
        let response = client.get("/api/status").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let items: Vec<LegacyStatus> = response.into_json().await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].st_title, "Draft");
    }

    #[rocket::async_test]
    async fn get_returns_specific_status() {
        let mut repo = DieselRepoMock::default();
        repo.requirement_statuses.insert(
            5,
            RequirementStatus {
                req_st_id: 5,
                req_st_title: "Approved".into(),
                req_st_description: "Ready".into(),
                req_st_short_name: "AP".into(),
            },
        );

        let client = client_with_repo(repo).await;
        let response = client.get("/api/status/5").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let value: Value = response.into_json().await.unwrap();
        assert_eq!(value.get("title"), Some(&Value::from("Approved")));
        assert_eq!(value.get("id"), Some(&Value::from(5)));
    }

    #[rocket::async_test]
    async fn create_returns_created_identifier() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let response = client
            .post("/api/status")
            .header(ContentType::JSON)
            .body(
                json!({
                    "req_st_title": "In Review",
                    "req_st_description": "Under evaluation",
                    "req_st_short_name": "IR"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(payload.get("status"), Some(&Value::from("ok")));
        assert_eq!(payload.get("id"), Some(&Value::from(1)));
    }
}
