use std::ops::Deref;

use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{async_trait, Request};

use crate::auth::{clear_session_cookie, read_session_user_id};
use crate::logger::LogCtx;
use crate::models::User;
use crate::repository::errors::RepoError;
use crate::repository::{DieselCachedRepo, UserRepository};

/// Request guard that ensures the user is authenticated and loaded from the database.
pub struct SessionUser(pub User);

impl SessionUser {
    pub fn into_inner(self) -> User {
        self.0
    }
}

impl Deref for SessionUser {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for SessionUser {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let cookies = request.cookies();

        let user_id = match read_session_user_id(cookies) {
            Some(id) => id,
            None => {
                clear_session_cookie(cookies);
                return Outcome::Error((Status::Unauthorized, ()));
            }
        };

        let result = rocket::tokio::task::spawn_blocking(move || {
            DieselCachedRepo::read().get_user_by_id(user_id)
        })
        .await;

        match result {
            Ok(Ok(user)) => Outcome::Success(SessionUser(user)),
            Ok(Err(RepoError::NotFound)) => {
                clear_session_cookie(cookies);
                Outcome::Error((Status::Unauthorized, ()))
            }
            Ok(Err(_)) => Outcome::Error((Status::InternalServerError, ())),
            Err(_) => Outcome::Error((Status::InternalServerError, ())),
        }
    }
}

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

/// Request guard exposing the authenticated user together with logging context.
pub struct ApiUser {
    user: User,
    log_ctx: LogCtx,
}

impl ApiUser {
    pub fn user(&self) -> &User {
        &self.user
    }

    pub fn log_ctx(&self) -> &LogCtx {
        &self.log_ctx
    }

    pub fn into_parts(self) -> (User, LogCtx) {
        (self.user, self.log_ctx)
    }

    #[cfg(test)]
    pub fn fake_admin() -> Self {
        let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let user = User {
            user_id: 1,
            user_username: "admin".into(),
            user_name: "Administrator".into(),
            user_email: "admin@example.com".into(),
            user_creation_date: epoch,
            user_last_login: epoch,
            user_password: "".into(),
            is_admin: true,
        };

        ApiUser {
            log_ctx: LogCtx::new(user.user_id),
            user,
        }
    }

    #[cfg(test)]
    fn from_test_request(request: &Request<'_>) -> Option<Self> {
        if request.headers().get_one("x-test-user").is_some() {
            Some(Self::fake_admin())
        } else {
            None
        }
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
        #[cfg(test)]
        if let Some(user) = Self::from_test_request(request) {
            return Outcome::Success(user);
        }

        match request.guard::<SessionUser>().await {
            Outcome::Success(session_user) => {
                let user = session_user.into_inner();
                let log_ctx = LogCtx::from_request(user.user_id, request);
                Outcome::Success(ApiUser { user, log_ctx })
            }
            Outcome::Error((status, ())) => Outcome::Error((status, ())),
            Outcome::Forward(status) => Outcome::Forward(status),
        }
    }
}
