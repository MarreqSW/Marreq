// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use std::ops::Deref;

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};

use crate::auth::guards::SessionUser;
use crate::models::User;

/// Request guard ensuring the current user has administrator privileges.
pub struct AdminOnly(pub User);

impl AdminOnly {
    pub fn into_inner(self) -> User {
        self.0
    }
}

impl Deref for AdminOnly {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for AdminOnly {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.guard::<SessionUser>().await {
            Outcome::Success(user) => {
                if user.is_admin {
                    Outcome::Success(AdminOnly(user.into_inner()))
                } else {
                    Outcome::Error((Status::Forbidden, ()))
                }
            }
            Outcome::Error((status, ())) => Outcome::Error((status, ())),
            Outcome::Forward(status) => Outcome::Forward(status),
        }
    }
}
