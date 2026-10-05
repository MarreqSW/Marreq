// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Dependency Structure Matrix API (issue #328): requirement × requirement
//! dependencies from requirement version links, with loop detection.

use crate::api::prelude::*;
use crate::auth::guards::ProjectRequirementsAndTraceabilityRead;
use crate::repository::errors::RepoError;
use crate::services::dsm_service::{self, Dsm, DsmOptions, DsmOrder};
use crate::services::requirement_service::REQUIREMENT_VERSION_LINK_TYPES;

/// Parse the query parameters shared by the view and the Excel export.
///
/// `link_types` is comma-separated; omitted means the defaults (all but
/// `RELATES_TO`) and an empty value means none.
pub fn dsm_options(
    link_types: Option<&str>,
    category_id: Option<i32>,
    root_id: Option<i32>,
    order: Option<&str>,
) -> ApiResult<DsmOptions> {
    let link_types = match link_types {
        None => dsm_service::default_link_types(),
        Some(raw) => {
            let mut types = Vec::new();
            for t in raw.split(',').map(str::trim).filter(|t| !t.is_empty()) {
                if !REQUIREMENT_VERSION_LINK_TYPES.contains(&t) {
                    return Err(ApiError::BadRequest(format!("unknown link type: {t}")));
                }
                if !types.iter().any(|x: &String| x == t) {
                    types.push(t.to_string());
                }
            }
            types
        }
    };
    let order = match order {
        None => DsmOrder::default(),
        Some(raw) => DsmOrder::parse(raw).ok_or_else(|| {
            ApiError::BadRequest(format!("unknown order: {raw} (use hierarchy or partition)"))
        })?,
    };
    Ok(DsmOptions {
        link_types,
        category_id,
        root_id,
        order,
    })
}

/// Build the matrix for a project; a `root_id` outside the project is a 404.
pub fn load(state: &State<AppState>, project_id: i32, options: &DsmOptions) -> ApiResult<Dsm> {
    let repo = state.repo_read();
    dsm_service::load_dsm(&*repo, project_id, options).map_err(|e| match e {
        RepoError::NotFound => ApiError::NotFound("root requirement not found in project".into()),
        other => other.into(),
    })
}

/// `GET /api/projects/<id>/dsm` — ordered requirements, non-empty cells, groups and loops.
#[get("/projects/<project_id>/dsm?<link_types>&<category_id>&<root_id>&<order>")]
pub async fn get_dsm(
    access: ProjectRequirementsAndTraceabilityRead,
    project_id: i32,
    link_types: Option<&str>,
    category_id: Option<i32>,
    root_id: Option<i32>,
    order: Option<&str>,
    state: &State<AppState>,
) -> ApiResult<Json<Dsm>> {
    require_project_permission(
        state,
        access.user(),
        project_id,
        Permission::ViewRequirements,
    )?;
    let options = dsm_options(link_types, category_id, root_id, order)?;
    Ok(Json(load(state, project_id, &options)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::test_session_cookie_for;
    use crate::models::{
        Project, ProjectMember, Requirement, RequirementVersion, RequirementVersionLink,
    };
    use crate::repository::{CacheRepository, diesel_repo_mock::DieselRepoMock};
    use crate::status_enums::ProjectStatus;
    use chrono::NaiveDate;
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;
    use rocket::serde::json::Value;
    use std::sync::{Arc, RwLock};

    type TestState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;
    const OUTSIDER_ID: i32 = 2;
    const PROJECT_ID: i32 = 1;

    fn epoch() -> chrono::NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn requirement(id: i32, code: &str) -> Requirement {
        Requirement {
            id,
            current_version_id: Some(id * 10),
            same_as_current: None,
            title: format!("Req {id}"),
            description: String::new(),
            status_id: 1,
            author_id: ADMIN_ID,
            reviewer_id: ADMIN_ID,
            reference_code: code.into(),
            category_id: 1,
            parent_id: None,
            creation_date: epoch(),
            update_date: epoch(),
            deadline_date: None,
            applicability_id: 1,
            justification: None,
            project_id: PROJECT_ID,
            approval_state: "draft".into(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        }
    }

    fn version(id: i32, requirement_id: i32) -> RequirementVersion {
        RequirementVersion {
            id,
            requirement_id,
            title: format!("Req {requirement_id}"),
            description: String::new(),
            status_id: 1,
            author_id: ADMIN_ID,
            reviewer_id: ADMIN_ID,
            category_id: 1,
            applicability_id: 1,
            justification: None,
            deadline_date: None,
            created_at: epoch(),
            approval_state: "draft".into(),
            approved_by: None,
            approved_at: None,
            reviewed_by: None,
            reviewed_at: None,
        }
    }

    fn link(id: i32, source_req: i32, target_req: i32, link_type: &str) -> RequirementVersionLink {
        RequirementVersionLink {
            id,
            source_version_id: source_req * 10,
            target_version_id: target_req * 10,
            link_type: link_type.into(),
            rationale: None,
            project_id: PROJECT_ID,
            created_at: epoch(),
            metadata: None,
        }
    }

    fn repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default().with_admin_user();
        repo.users.insert(
            OUTSIDER_ID,
            DieselRepoMock::make_user(OUTSIDER_ID, "bob", ""),
        );
        repo.projects.insert(
            PROJECT_ID,
            Project {
                id: PROJECT_ID,
                name: "P".into(),
                description: None,
                creation_date: None,
                update_date: None,
                status: ProjectStatus::Active,
                owner_id: Some(ADMIN_ID),
                slug: "p".into(),
                group_id: None,
                archived_at: None,
                archived_by: None,
            },
        );
        repo.project_members.push(ProjectMember {
            project_id: PROJECT_ID,
            user_id: ADMIN_ID,
            role: 1,
            created_at: epoch(),
            updated_at: epoch(),
        });
        for (id, code) in [(1, "REQ-1"), (2, "REQ-2"), (3, "REQ-3")] {
            repo.requirements.insert(id, requirement(id, code));
            repo.requirement_versions
                .insert(id * 10, version(id * 10, id));
        }
        repo.requirement_version_links = vec![
            link(1, 2, 3, "DEPENDS_ON"),
            link(2, 3, 2, "DEPENDS_ON"),
            link(3, 3, 1, "RELATES_TO"),
        ];
        repo.next_link_id = 4;
        repo
    }

    async fn client() -> Client {
        let state: TestState = AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo(), 0))),
        };
        let rocket = rocket::build()
            .manage(state)
            .mount("/api", routes![get_dsm]);
        Client::tracked(rocket).await.unwrap()
    }

    fn cookie(client: &Client, user_id: i32) -> rocket::http::Cookie<'static> {
        test_session_cookie_for(client.rocket().state::<TestState>().unwrap(), user_id)
    }

    #[rocket::async_test]
    async fn returns_matrix_with_loop_and_default_link_types() {
        let client = client().await;
        let res = client
            .get(format!("/api/projects/{PROJECT_ID}/dsm"))
            .private_cookie(cookie(&client, ADMIN_ID))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);
        let body: Value = res.into_json().await.unwrap();
        assert_eq!(body["requirements"].as_array().unwrap().len(), 3);
        // RELATES_TO is off by default, so only the two DEPENDS_ON cells.
        assert_eq!(body["cells"].as_array().unwrap().len(), 2);
        assert_eq!(body["loops"].as_array().unwrap().len(), 1);
        assert_eq!(body["stats"]["loops"], 1);
        assert_eq!(body["order"], "hierarchy");
    }

    #[rocket::async_test]
    async fn filters_by_link_type_and_orders_by_partition() {
        let client = client().await;
        let res = client
            .get(format!(
                "/api/projects/{PROJECT_ID}/dsm?link_types=RELATES_TO&order=partition"
            ))
            .private_cookie(cookie(&client, ADMIN_ID))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);
        let body: Value = res.into_json().await.unwrap();
        assert_eq!(body["cells"].as_array().unwrap().len(), 1);
        assert_eq!(body["order"], "partition");
        assert!(body["groups"].as_array().unwrap().is_empty());
    }

    #[rocket::async_test]
    async fn rejects_unknown_link_type_and_order() {
        let client = client().await;
        for query in ["link_types=NOPE", "order=random"] {
            let res = client
                .get(format!("/api/projects/{PROJECT_ID}/dsm?{query}"))
                .private_cookie(cookie(&client, ADMIN_ID))
                .dispatch()
                .await;
            assert_eq!(res.status(), Status::BadRequest, "{query}");
        }
    }

    #[rocket::async_test]
    async fn root_outside_project_is_not_found() {
        let client = client().await;
        let res = client
            .get(format!("/api/projects/{PROJECT_ID}/dsm?root_id=999"))
            .private_cookie(cookie(&client, ADMIN_ID))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn requires_login_and_membership() {
        let client = client().await;
        let anonymous = client
            .get(format!("/api/projects/{PROJECT_ID}/dsm"))
            .dispatch()
            .await;
        assert_eq!(anonymous.status(), Status::Unauthorized);
        let outsider = client
            .get(format!("/api/projects/{PROJECT_ID}/dsm"))
            .private_cookie(cookie(&client, OUTSIDER_ID))
            .dispatch()
            .await;
        assert_eq!(outsider.status(), Status::Forbidden);
    }
}
