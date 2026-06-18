// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use std::ops::Deref;

use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};

use crate::auth::guards::SessionUser;
use crate::logger::LogCtx;
use crate::models::User;

/// Request guard exposing the authenticated user together with logging context.
pub struct ApiUser {
    user: User,
    log_ctx: LogCtx,
}

impl ApiUser {
    pub(crate) fn new(user: User, log_ctx: LogCtx) -> Self {
        Self { user, log_ctx }
    }

    pub fn user(&self) -> &User {
        &self.user
    }

    pub fn log_ctx(&self) -> &LogCtx {
        &self.log_ctx
    }

    pub fn into_parts(self) -> (User, LogCtx) {
        (self.user, self.log_ctx)
    }
}

impl Deref for ApiUser {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.user
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for ApiUser {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match request.guard::<SessionUser>().await {
            Outcome::Success(session_user) => {
                let user = session_user.into_inner();
                let log_ctx = LogCtx::from_request(user.id, request);
                Outcome::Success(ApiUser { user, log_ctx })
            }
            Outcome::Error((status, ())) => Outcome::Error((status, ())),
            Outcome::Forward(status) => Outcome::Forward(status),
        }
    }
}
