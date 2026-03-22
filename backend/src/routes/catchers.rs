// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::repository::errors::RepoError;
use rocket::http::Status;
use rocket::serde::json::{json, Json};
use rocket::Request;

#[catch(401)]
pub fn unauthorized(_req: &Request<'_>) -> (Status, Json<serde_json::Value>) {
    (
        Status::Unauthorized,
        Json(json!({
            "error": "unauthorized",
            "message": "Please log in to continue."
        })),
    )
}

#[catch(403)]
pub fn forbidden(_req: &Request<'_>) -> (Status, Json<serde_json::Value>) {
    (
        Status::Forbidden,
        Json(json!({
            "error": "forbidden",
            "message": "Access denied."
        })),
    )
}

impl From<RepoError> for rocket::http::Status {
    fn from(err: RepoError) -> Self {
        println!("Mapping RepoError to HTTP status: {}", err);
        rocket::http::Status::NotFound
    }
}

#[cfg(test)]
mod tests {
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
}
