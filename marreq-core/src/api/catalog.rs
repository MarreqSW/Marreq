use crate::api::prelude::*;
use crate::auth::guards::ProjectRequirementsRead;
use crate::repository::{CustomFieldRepository, LookupRepository};

#[get("/projects/<project_id>/catalog")]
pub fn get(
    access: ProjectRequirementsRead,
    project_id: i32,
    state: &State<AppState>,
) -> ApiResult<Json<Value>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let repo = state.repo_read();
    Ok(Json(json!({
        "categories": repo.get_categories_by_project(project_id)?,
        "applicability": repo.get_applicability_by_project(project_id)?,
        "requirement_statuses": repo.get_requirement_status_by_project(project_id)?,
        "verification_statuses": repo.get_verification_status_by_project(project_id)?,
        "verification_methods": repo.get_verification_methods_by_project(project_id)?,
        "custom_fields": repo.list_custom_field_definitions_by_project(project_id)?,
    })))
}
