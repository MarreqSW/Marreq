// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! API endpoints for user notifications and notification preferences.

use crate::api::prelude::*;
use crate::models::NewNotificationPreference;
use crate::services::NotificationService;
use rocket::serde::Deserialize;

#[get("/notifications?<unread_only>&<limit>")]
pub fn list(
    user: ApiUser,
    state: &State<AppState>,
    unread_only: Option<bool>,
    limit: Option<i64>,
) -> ApiResult<Value> {
    let service = NotificationService::new(state.inner());
    let items = service.list_for_user(
        user.user().id,
        limit.unwrap_or(50),
        unread_only.unwrap_or(false),
    )?;
    Ok(json!(items))
}

#[get("/notifications/unread-count")]
pub fn unread_count(user: ApiUser, state: &State<AppState>) -> ApiResult<Value> {
    let service = NotificationService::new(state.inner());
    let count = service.unread_count(user.user().id)?;
    Ok(json!({ "count": count }))
}

#[patch("/notifications/<id>/read")]
pub fn mark_read(user: ApiUser, state: &State<AppState>, id: i32) -> ApiResult<Value> {
    let service = NotificationService::new(state.inner());
    let updated = service.mark_read(id, user.user().id)?;
    Ok(json!({ "success": updated }))
}

#[post("/notifications/read-all")]
pub fn mark_all_read(user: ApiUser, state: &State<AppState>) -> ApiResult<Value> {
    let service = NotificationService::new(state.inner());
    let count = service.mark_all_read(user.user().id)?;
    Ok(json!({ "count": count }))
}

#[get("/notifications/preferences")]
pub fn get_preferences(user: ApiUser, state: &State<AppState>) -> ApiResult<Value> {
    let service = NotificationService::new(state.inner());
    let prefs = service.get_preferences(user.user().id)?;
    Ok(json!(prefs))
}

#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct PreferencePayload {
    pub notify_in_app: Option<bool>,
    pub notify_email: Option<bool>,
}

#[put("/notifications/preferences/<project_id>", data = "<payload>")]
pub fn set_preference(
    user: ApiUser,
    state: &State<AppState>,
    project_id: i32,
    payload: Json<PreferencePayload>,
) -> ApiResult<Value> {
    let service = NotificationService::new(state.inner());
    service.set_preference(&NewNotificationPreference {
        user_id: user.user().id,
        project_id,
        notify_in_app: payload.notify_in_app.unwrap_or(true),
        notify_email: payload.notify_email.unwrap_or(false),
    })?;
    Ok(json!({ "status": "ok" }))
}

#[delete("/notifications/preferences/<project_id>")]
pub fn delete_preference(
    user: ApiUser,
    state: &State<AppState>,
    project_id: i32,
) -> ApiResult<Value> {
    let service = NotificationService::new(state.inner());
    service.delete_preference(user.user().id, project_id)?;
    Ok(json!({ "status": "ok" }))
}
