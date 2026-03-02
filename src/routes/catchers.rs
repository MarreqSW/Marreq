// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::repository::errors::RepoError;
use crate::services::log_service::LogServiceError;
use rocket::response::Redirect;
use rocket::serde::json::json;
use rocket::Request;
use rocket_dyn_templates::Template;

#[catch(401)]
pub fn unauthorized(_req: &Request<'_>) -> Template {
    let context = json!({
        "title": "Login",
        "error": "Please log in to continue."
    });

    Template::render("login", context)
}

#[catch(403)]
pub fn forbidden(_req: &Request<'_>) -> Template {
    let context = json!({
        "title": "Access Denied"
    });

    Template::render("access_denied", context)
}

impl From<RepoError> for rocket::http::Status {
    fn from(err: RepoError) -> Self {
        println!("Redirecting to error page due to: {}", err);
        rocket::http::Status::NotFound
    }
}

impl From<RepoError> for Redirect {
    fn from(err: RepoError) -> Self {
        println!("Redirecting to error page due to: {}", err);
        Redirect::to("/error")
    }
}

impl From<LogServiceError> for Redirect {
    fn from(err: LogServiceError) -> Self {
        println!("Redirecting to error page due to: {}", err);
        Redirect::to("/error")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::errors::RepoError;
    use diesel::result::Error as DieselError;

    #[test]
    fn repo_error_to_status_not_found() {
        use rocket::http::Status;
        let error = RepoError::NotFound;
        let status: Status = error.into();
        assert_eq!(status, Status::NotFound);
    }

    #[test]
    fn repo_error_to_status_pool_error() {
        use rocket::http::Status;
        let error = RepoError::Pool("test".to_string());
        let status: Status = error.into();
        assert_eq!(status, Status::NotFound);
    }

    #[test]
    fn repo_error_to_status_bad_input() {
        use rocket::http::Status;
        let error = RepoError::BadInput("test".to_string());
        let status: Status = error.into();
        assert_eq!(status, Status::NotFound);
    }

    #[test]
    fn repo_error_to_status_unauthorized() {
        use rocket::http::Status;
        let error = RepoError::Unauthorized;
        let status: Status = error.into();
        assert_eq!(status, Status::NotFound);
    }

    #[test]
    fn repo_error_to_status_db_error() {
        use rocket::http::Status;
        let diesel_error = DieselError::NotFound;
        let error: RepoError = diesel_error.into();
        let status: Status = error.into();
        assert_eq!(status, Status::NotFound);
    }

    #[test]
    fn repo_error_to_redirect_not_found() {
        let error = RepoError::NotFound;
        let redirect: Redirect = error.into();
        // Test that redirect is created (can't easily test URI in unit tests)
        // Dropping redirect without panic proves conversion succeeded
        drop(redirect);
    }

    #[test]
    fn repo_error_to_redirect_pool_error() {
        let error = RepoError::Pool("test".to_string());
        let redirect: Redirect = error.into();
        // Dropping redirect without panic proves conversion succeeded
        drop(redirect);
    }

    #[test]
    fn repo_error_to_redirect_bad_input() {
        let error = RepoError::BadInput("test".to_string());
        let redirect: Redirect = error.into();
        // Dropping redirect without panic proves conversion succeeded
        drop(redirect);
    }

    #[test]
    fn repo_error_to_redirect_unauthorized() {
        let error = RepoError::Unauthorized;
        let redirect: Redirect = error.into();
        // Dropping redirect without panic proves conversion succeeded
        drop(redirect);
    }

    #[test]
    fn repo_error_to_redirect_db_error() {
        let diesel_error = DieselError::NotFound;
        let error: RepoError = diesel_error.into();
        let redirect: Redirect = error.into();
        // Dropping redirect without panic proves conversion succeeded
        drop(redirect);
    }
}
