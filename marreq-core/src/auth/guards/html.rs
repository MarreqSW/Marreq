// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use std::ops::Deref;

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};

use crate::app::AppState;
use crate::auth::guards::route_params::extract_route_param;
use crate::auth::guards::session::session_user_has_project_access;
use crate::auth::guards::SessionUser;
use crate::models::User;
use crate::namespaces::{
    project_namespace_segment, resolve_project_namespace_entity, NamespaceEntity,
};
use crate::repository::errors::RepoError;
use crate::repository::ProjectsRepository;

/// Request guard ensuring the authenticated user may access the requested HTML project slug.
pub struct HtmlProjectAccess {
    user: User,
    project_id: i32,
    namespace: String,
    project_slug: String,
    project_route_slug: String,
}

impl HtmlProjectAccess {
    pub fn user(&self) -> &User {
        &self.user
    }

    pub fn project_id(&self) -> i32 {
        self.project_id
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn project_slug(&self) -> &str {
        &self.project_slug
    }

    pub fn project_route_slug(&self) -> &str {
        &self.project_route_slug
    }

    pub fn into_user(self) -> User {
        self.user
    }

    pub fn into_parts(self) -> (User, i32, String, String, String) {
        (
            self.user,
            self.project_id,
            self.namespace,
            self.project_slug,
            self.project_route_slug,
        )
    }
}

impl Deref for HtmlProjectAccess {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.user
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for HtmlProjectAccess {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let namespace = match extract_route_param(request, "<namespace>") {
            Ok(segment) => segment,
            Err(status) => return Outcome::Error((status, ())),
        };
        let requested_project_slug = match extract_route_param(request, "<project_id>") {
            Ok(segment) => segment,
            Err(status) => return Outcome::Error((status, ())),
        };

        let state = match request.rocket().state::<AppState>() {
            Some(state) => state,
            None => return Outcome::Error((Status::InternalServerError, ())),
        };

        let project = {
            let repo = match state.try_repo_read() {
                Ok(repo) => repo,
                Err(_) => return Outcome::Error((Status::InternalServerError, ())),
            };

            let namespace_entity = match resolve_project_namespace_entity(&*repo, &namespace) {
                Ok(entity) => entity,
                Err(RepoError::NotFound) => return Outcome::Error((Status::NotFound, ())),
                Err(_) => return Outcome::Error((Status::InternalServerError, ())),
            };

            let lookup = match &namespace_entity {
                NamespaceEntity::User(user) => repo.get_project_by_user_namespace_and_slug(
                    &user.username,
                    &requested_project_slug,
                ),
                NamespaceEntity::Group(group) => repo
                    .get_project_by_group_namespace_and_slug(&group.slug, &requested_project_slug),
            };

            match lookup {
                Ok(project) => project,
                Err(RepoError::NotFound) => return Outcome::Error((Status::NotFound, ())),
                Err(_) => return Outcome::Error((Status::InternalServerError, ())),
            }
        };

        match request.guard::<SessionUser>().await {
            Outcome::Success(session_user) => {
                let user = session_user.into_inner();
                let namespace = {
                    let repo = match state.try_repo_read() {
                        Ok(repo) => repo,
                        Err(_) => return Outcome::Error((Status::InternalServerError, ())),
                    };

                    match project_namespace_segment(&*repo, &project) {
                        Ok(segment) => segment,
                        Err(_) => return Outcome::Error((Status::InternalServerError, ())),
                    }
                };
                let project_slug = project.slug.clone();
                let project_route_slug = format!("{namespace}/{project_slug}");
                match session_user_has_project_access(state, &user, project.id) {
                    Ok(true) => Outcome::Success(HtmlProjectAccess {
                        user,
                        project_id: project.id,
                        namespace,
                        project_slug,
                        project_route_slug,
                    }),
                    Ok(false) => Outcome::Error((Status::Forbidden, ())),
                    Err(_) => Outcome::Error((Status::InternalServerError, ())),
                }
            }
            Outcome::Error((status, ())) => Outcome::Error((status, ())),
            Outcome::Forward(status) => Outcome::Forward(status),
        }
    }
}
