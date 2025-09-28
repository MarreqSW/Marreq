use crate::api::prelude::*;
use crate::logger::{LogCtx, Logger};
use crate::models::{Applicability, NewApplicability};
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

    let id = state
        .repo
        .async_write(move |repo| {
            let id = repo.insert_new_applicability(&app)?;
            if let Ok(mut conn) = repo.inner_repo().get_conn() {
                let ctx = LogCtx::new(0);
                let _ = Logger::created(conn.as_mut(), &ctx, id, &app);
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

            if let Some(prev) = before {
                let after = Applicability {
                    app_id: id,
                    app_title: app.app_title.clone(),
                    app_description: app.app_description.clone(),
                    app_tag: app.app_tag.clone(),
                    project_id: app.project_id,
                };
                if let Ok(mut conn) = repo.inner_repo().get_conn() {
                    let ctx = LogCtx::new(0);
                    let _ = Logger::updated(conn.as_mut(), &ctx, &prev, &after);
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
                let ctx = LogCtx::new(0);
                let _ = Logger::deleted(conn.as_mut(), &ctx, &removed);
            }
            Ok::<_, RepoError>(())
        })
        .await?;
    Ok(Status::NoContent)
}
