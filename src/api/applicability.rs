use crate::api::prelude::*;
use crate::logger::Logger;
use crate::models::{Applicability, EntityType, NewApplicability};
use crate::repository::errors::RepoError;
use crate::repository::LookupRepository;

#[get("/applicability")]
pub async fn list(state: &State<AppState>) -> ApiResult<Json<Vec<Applicability>>> {
    let items = state
        .repo
        .async_read(|repo| repo.get_applicability_all())
        .await?;
    Ok(Json(items))
}

#[get("/applicability/<id>")]
pub async fn get(state: &State<AppState>, id: i32) -> ApiResult<Json<Applicability>> {
    let applicability = state
        .repo
        .async_read(move |repo| repo.get_applicability_by_id(id))
        .await?;

    Ok(Json(applicability))
}

#[post("/applicability", data = "<payload>")]
pub async fn create(
    state: &State<AppState>,
    payload: Json<NewApplicability>,
) -> ApiResult<(Status, Value)> {
    let app = payload.into_inner();
    let title = app.app_title.clone();
    let pid = app.project_id;
    let json_for_log = Logger::to_json_string(&app).ok();

    let id = state
        .repo
        .async_write(move |repo| {
            let id = repo.insert_new_applicability(&app)?;
            if let Some(payload) = json_for_log {
                if let Ok(mut conn) = repo.inner_repo().get_conn() {
                    let _ = Logger::log_create(
                        conn.as_mut(),
                        0,
                        EntityType::Applicability,
                        id,
                        Some(pid),
                        Some(payload),
                        Some(format!("Created applicability via API: {title}")),
                        None,
                    );
                }
            }
            Ok::<_, RepoError>(id)
        })
        .await?;

    Ok((Status::Created, json!({ "status": "ok", "id": id })))
}

#[put("/applicability/<id>", data = "<payload>")]
pub async fn update(
    state: &State<AppState>,
    id: i32,
    payload: Json<NewApplicability>,
) -> ApiResult<Value> {
    let mut app = payload.into_inner();
    app.app_id = Some(id);

    state
        .repo
        .async_write(move |repo| {
            let before = match repo.get_applicability_by_id(id) {
                Ok(a) => Some(a),
                Err(RepoError::NotFound) => None,
                Err(e) => return Err(e),
            };

            let updated = repo.edit_applicability(&app)?;
            if !updated {
                return Err(RepoError::NotFound);
            }

            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                if let Some(prev) = before {
                    if let (Ok(old_values), Ok(new_values)) =
                        (Logger::to_json_string(&prev), Logger::to_json_string(&app))
                    {
                        let _ = Logger::log_update(
                            conn.as_mut(),
                            0,
                            EntityType::Applicability,
                            id,
                            Some(app.project_id),
                            Some(old_values),
                            Some(new_values),
                            Some("Updated applicability via API".into()),
                            None,
                        );
                    }
                }
            }

            Ok::<_, RepoError>(())
        })
        .await?;

    Ok(json!({
        "status": "ok",
        "message": "Applicability updated successfully"
    }))
}

#[delete("/applicability/<id>")]
pub async fn delete(state: &State<AppState>, id: i32) -> ApiResult<Status> {
    state
        .repo
        .async_write(move |repo| {
            let removed = repo.delete_applicability(id)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                if let Ok(old_values) = Logger::to_json_string(&removed) {
                    let _ = Logger::log_delete(
                        conn.as_mut(),
                        0,
                        EntityType::Applicability,
                        id,
                        Some(removed.project_id),
                        Some(old_values),
                        Some(format!(
                            "Deleted applicability via API: {}",
                            removed.app_title
                        )),
                        None,
                    );
                }
            }
            Ok::<_, RepoError>(())
        })
        .await?;
    Ok(Status::NoContent)
}
