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

/// Rocket’s built-in 404 page is HTML and confuses people who open SPA paths on the API port (`:8000`).
/// This catcher only runs when **no route matches** (not when a handler returns its own 404 JSON).
#[catch(404)]
pub fn not_found(req: &Request<'_>) -> (Status, Json<serde_json::Value>) {
    let path = req.uri().path().as_str();
    let message = if path.starts_with("/p/") {
        "This path looks like a React app route. This process is the JSON API only (GET / and /api/*). Open the UI on the Vite dev server (http://127.0.0.1:5173) or your nginx/frontend URL — not the API port."
    } else if path.starts_with("/api/") {
        "No API route matched this path and method."
    } else {
        "No route matched. The Marreq web UI is served separately from this API (e.g. Vite :5173 or the docker frontend container)."
    };
    (
        Status::NotFound,
        Json(json!({
            "error": "not_found",
            "message": message,
            "path": path,
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
