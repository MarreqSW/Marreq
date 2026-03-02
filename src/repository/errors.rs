// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use diesel::result::Error as DieselError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepoError {
    #[error("not found")]
    NotFound,
    #[error("database error: {0}")]
    Db(#[from] DieselError),
    #[error("pool error: {0}")]
    Pool(String),
    #[error("bad input: {0}")]
    BadInput(String),
    #[error("unauthorized")]
    Unauthorized,
    /// A uniqueness constraint was violated (e.g. duplicate username or email).
    #[error("duplicate: {0}")]
    Duplicate(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_error_not_found_display() {
        let error = RepoError::NotFound;
        let display = format!("{}", error);
        assert_eq!(display, "not found");
    }

    #[test]
    fn repo_error_pool_display() {
        let error = RepoError::Pool("test error".to_string());
        let display = format!("{}", error);
        assert_eq!(display, "pool error: test error");
    }

    #[test]
    fn repo_error_bad_input_display() {
        let error = RepoError::BadInput("invalid input".to_string());
        let display = format!("{}", error);
        assert_eq!(display, "bad input: invalid input");
    }

    #[test]
    fn repo_error_unauthorized_display() {
        let error = RepoError::Unauthorized;
        let display = format!("{}", error);
        assert_eq!(display, "unauthorized");
    }

    #[test]
    fn repo_error_db_from_diesel_error() {
        let diesel_error = DieselError::NotFound;
        let repo_error: RepoError = diesel_error.into();
        match repo_error {
            RepoError::Db(_) => {}
            _ => panic!("Expected Db variant"),
        }
    }

    #[test]
    fn repo_error_debug() {
        let error = RepoError::NotFound;
        let debug = format!("{:?}", error);
        assert!(debug.contains("NotFound"));
    }

    #[test]
    fn repo_error_pool_with_empty_string() {
        let error = RepoError::Pool("".to_string());
        let display = format!("{}", error);
        assert_eq!(display, "pool error: ");
    }

    #[test]
    fn repo_error_bad_input_with_empty_string() {
        let error = RepoError::BadInput("".to_string());
        let display = format!("{}", error);
        assert_eq!(display, "bad input: ");
    }

    #[test]
    fn repo_error_pool_with_long_message() {
        let message = "A".repeat(1000);
        let error = RepoError::Pool(message.clone());
        let display = format!("{}", error);
        assert_eq!(display, format!("pool error: {}", message));
    }

    #[test]
    fn repo_error_bad_input_with_long_message() {
        let message = "A".repeat(1000);
        let error = RepoError::BadInput(message.clone());
        let display = format!("{}", error);
        assert_eq!(display, format!("bad input: {}", message));
    }
}
