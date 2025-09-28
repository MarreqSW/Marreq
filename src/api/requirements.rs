use rocket::serde::Deserialize;

use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{NewRequirement, Requirement};
use crate::repository::errors::RepoError;
use crate::repository::RequirementsRepository;

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
pub struct RequirementPatch {
    pub req_title: Option<String>,
    pub req_description: Option<String>,
    pub req_current_status: Option<i32>,
    pub req_verification: Option<i32>,
    pub req_author: Option<i32>,
    pub req_reviewer: Option<i32>,
    pub req_category: Option<i32>,
    pub req_applicability: Option<i32>,
}

#[get("/requirements")]
pub async fn list(_user: ApiUser, state: &State<AppState>) -> ApiResult<Json<Vec<Requirement>>> {
    let requirements = state
        .repo
        .async_read(|repo| repo.get_requirements_all())
        .await?;
    Ok(Json(requirements))
}

#[get("/requirements/<id>")]
pub async fn get(_user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Json<Requirement>> {
    let requirement = state
        .repo
        .async_read(move |repo| repo.get_requirement_by_id(id))
        .await?;
    Ok(Json(requirement))
}

#[post("/requirements", data = "<payload>")]
pub async fn create(
    user: ApiUser,
    state: &State<AppState>,
    payload: Json<NewRequirement>,
) -> ApiResult<Value> {
    let requirement = payload.into_inner();
    let log_ctx = user.log_ctx().clone();

    let id = state
        .repo
        .async_write(move |repo| {
            let id = repo.insert_new_requirement(&requirement)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                let _ = Logger::created(conn.as_mut(), &log_ctx, id, &requirement);
            }
            Ok::<_, RepoError>(id)
        })
        .await?;

    Ok(json!({ "status": "ok", "id": id }))
}

#[delete("/requirements/<id>")]
pub async fn delete(user: ApiUser, id: i32, state: &State<AppState>) -> ApiResult<Status> {
    let log_ctx = user.log_ctx().clone();
    state
        .repo
        .async_write(move |repo| {
            let removed = repo.delete_requirement(id)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                let _ = Logger::deleted(conn.as_mut(), &log_ctx, &removed);
            }
            Ok::<_, RepoError>(())
        })
        .await?;
    Ok(Status::NoContent)
}

#[patch("/requirements/<id>", data = "<patch>")]
pub async fn patch_requirement(
    user: ApiUser,
    state: &State<AppState>,
    id: i32,
    patch: Json<RequirementPatch>,
) -> ApiResult<Value> {
    let patch = patch.into_inner();
    let log_ctx = user.log_ctx().clone();

    let any_updates = patch.req_title.is_some()
        || patch.req_description.is_some()
        || patch.req_current_status.is_some()
        || patch.req_verification.is_some()
        || patch.req_author.is_some()
        || patch.req_reviewer.is_some()
        || patch.req_category.is_some()
        || patch.req_applicability.is_some();

    if !any_updates {
        return Err(ApiError::BadRequest("no fields provided".into()));
    }

    state
        .repo
        .async_write(move |repo| {
            let mut requirement = repo.get_requirement_by_id(id)?;
            let original = requirement.clone();

            if let Some(v) = patch.req_title {
                requirement.req_title = v;
            }
            if let Some(v) = patch.req_description {
                requirement.req_description = v;
            }
            if let Some(v) = patch.req_current_status {
                requirement.req_current_status = v;
            }
            if let Some(v) = patch.req_verification {
                requirement.req_verification = v;
            }
            if let Some(v) = patch.req_author {
                requirement.req_author = v;
            }
            if let Some(v) = patch.req_reviewer {
                requirement.req_reviewer = v;
            }
            if let Some(v) = patch.req_category {
                requirement.req_category = v;
            }
            if let Some(v) = patch.req_applicability {
                requirement.req_applicability = v;
            }

            let payload = NewRequirement {
                req_id: Some(requirement.req_id),
                req_title: requirement.req_title.clone(),
                req_description: requirement.req_description.clone(),
                req_verification: requirement.req_verification,
                req_author: requirement.req_author,
                req_link: requirement.req_link.clone(),
                req_category: requirement.req_category,
                req_current_status: requirement.req_current_status,
                req_parent: requirement.req_parent,
                req_reference: requirement.req_reference.clone(),
                req_reviewer: requirement.req_reviewer,
                req_applicability: requirement.req_applicability,
                req_justification: requirement.req_justification.clone(),
                project_id: requirement.project_id,
            };

            repo.edit_requirement(&payload)?;

            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                let _ = Logger::updated(conn.as_mut(), &log_ctx, &original, &requirement);
            }

            Ok::<_, RepoError>(())
        })
        .await?;

    Ok(json!({
        "success": true,
        "message": "Field updated successfully"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use rocket::http::{ContentType, Cookie};
    use rocket::local::asynchronous::Client;
    use serde_json::{json, Value};
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;

    fn state_from_repo(repo: DieselRepoMock) -> TestState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn client_with_repo(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(state_from_repo(repo.with_admin_user()))
            .mount(
                "/api",
                routes![list, get, create, delete, patch_requirement],
            );
        Client::tracked(rocket).await.unwrap()
    }

    fn auth_cookie() -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, ADMIN_ID.to_string());
        cookie.set_path("/");
        cookie
    }

    fn sample_requirement(title: &str) -> Value {
        json!({
            "req_id": null,
            "req_title": title,
            "req_description": format!("{title} description"),
            "req_verification": 1,
            "req_author": 1,
            "req_link": "http://example.com",
            "req_category": 1,
            "req_current_status": 1,
            "req_parent": 0,
            "req_reference": "REF-1",
            "req_reviewer": 2,
            "req_applicability": 3,
            "req_justification": null,
            "project_id": 1
        })
    }

    #[rocket::async_test]
    async fn list_returns_empty_array() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let response = client
            .get("/api/requirements")
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let items: Vec<Requirement> = response.into_json().await.unwrap();
        assert!(items.is_empty());
    }

    #[rocket::async_test]
    async fn create_returns_identifier() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let response = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(sample_requirement("First").to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(payload.get("status"), Some(&Value::from("ok")));
        assert_eq!(payload.get("id"), Some(&Value::from(1)));
    }

    #[rocket::async_test]
    async fn patch_updates_fields() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create_response = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(sample_requirement("Original").to_string())
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let response = client
            .patch(format!("/api/requirements/{id}"))
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(
                json!({
                    "req_title": "Updated",
                    "req_description": "Updated description"
                })
                .to_string(),
            )
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let payload: Value = response.into_json().await.unwrap();
        assert_eq!(payload.get("success"), Some(&Value::from(true)));

        let get_response = client
            .get(format!("/api/requirements/{id}"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        let requirement: Requirement = get_response.into_json().await.unwrap();
        assert_eq!(requirement.req_title, "Updated");
        assert_eq!(requirement.req_description, "Updated description");
    }

    #[rocket::async_test]
    async fn delete_removes_requirement() {
        let client = client_with_repo(DieselRepoMock::default()).await;
        let create_response = client
            .post("/api/requirements")
            .header(ContentType::JSON)
            .private_cookie(auth_cookie())
            .body(sample_requirement("Disposable").to_string())
            .dispatch()
            .await;
        let created: Value = create_response.into_json().await.unwrap();
        let id = created.get("id").and_then(Value::as_i64).unwrap() as i32;

        let delete_response = client
            .delete(format!("/api/requirements/{id}"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(delete_response.status(), Status::NoContent);

        let not_found = client
            .get(format!("/api/requirements/{id}"))
            .private_cookie(auth_cookie())
            .dispatch()
            .await;
        assert_eq!(not_found.status(), Status::NotFound);
    }
}
