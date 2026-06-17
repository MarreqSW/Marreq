// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use std::ops::Deref;

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};

use crate::app::AppState;
use crate::auth::guards::route_params::extract_route_param;
use crate::auth::guards::session::session_user_has_group_access;
use crate::auth::guards::SessionUser;
use crate::models::User;
use crate::permissions::{has_group_permission, GroupPermission};
use crate::repository::errors::RepoError;
use crate::repository::GroupsRepository;

/// Request guard that ensures the user can view the addressed group page.
pub struct HtmlGroupAccess {
    user: User,
    group_id: i32,
    group_slug: String,
}

impl HtmlGroupAccess {
    pub fn user(&self) -> &User {
        &self.user
    }

    pub fn group_id(&self) -> i32 {
        self.group_id
    }

    pub fn group_slug(&self) -> &str {
        &self.group_slug
    }

    pub fn into_user(self) -> User {
        self.user
    }

    pub fn into_parts(self) -> (User, i32, String) {
        (self.user, self.group_id, self.group_slug)
    }
}

impl Deref for HtmlGroupAccess {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.user
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for HtmlGroupAccess {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let group_slug = match extract_route_param(request, "<group_slug>") {
            Ok(segment) => segment,
            Err(status) => return Outcome::Error((status, ())),
        };

        let state = match request.rocket().state::<AppState>() {
            Some(state) => state,
            None => return Outcome::Error((Status::InternalServerError, ())),
        };

        let group = {
            let repo = match state.try_repo_read() {
                Ok(repo) => repo,
                Err(_) => return Outcome::Error((Status::InternalServerError, ())),
            };

            match repo.get_group_by_slug(&group_slug) {
                Ok(group) => group,
                Err(RepoError::NotFound) => return Outcome::Error((Status::NotFound, ())),
                Err(_) => return Outcome::Error((Status::InternalServerError, ())),
            }
        };

        match request.guard::<SessionUser>().await {
            Outcome::Success(session_user) => {
                let user = session_user.into_inner();
                match session_user_has_group_access(state, &user, group.id) {
                    Ok(true) => Outcome::Success(HtmlGroupAccess {
                        user,
                        group_id: group.id,
                        group_slug,
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

/// Request guard that ensures the user can manage the addressed group's members/settings.
pub struct HtmlGroupManageAccess(pub HtmlGroupAccess);

impl HtmlGroupManageAccess {
    pub fn group_id(&self) -> i32 {
        self.0.group_id()
    }

    pub fn group_slug(&self) -> &str {
        self.0.group_slug()
    }

    pub fn into_parts(self) -> (User, i32, String) {
        self.0.into_parts()
    }
}

impl Deref for HtmlGroupManageAccess {
    type Target = HtmlGroupAccess;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for HtmlGroupManageAccess {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let access = match request.guard::<HtmlGroupAccess>().await {
            Outcome::Success(access) => access,
            Outcome::Error((status, ())) => return Outcome::Error((status, ())),
            Outcome::Forward(status) => return Outcome::Forward(status),
        };

        let state = match request.rocket().state::<AppState>() {
            Some(state) => state,
            None => return Outcome::Error((Status::InternalServerError, ())),
        };

        let repo = match state.try_repo_read() {
            Ok(repo) => repo,
            Err(_) => return Outcome::Error((Status::InternalServerError, ())),
        };

        if has_group_permission(
            &*repo,
            access.user(),
            access.group_id(),
            GroupPermission::ManageGroupMembers,
        ) {
            Outcome::Success(HtmlGroupManageAccess(access))
        } else {
            Outcome::Error((Status::Forbidden, ()))
        }
    }
}
