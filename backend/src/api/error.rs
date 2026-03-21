// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use rocket::http::Status;
use rocket::response::{Responder, Response};
use rocket::serde::json::json;
use rocket::Request;

use diesel::result::{DatabaseErrorKind, Error as DieselError};

use crate::repository::errors::RepoError;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Unauthorized(String),
    NotFound(String),
    Forbidden(String),
    Conflict(String),
    /// Returned when a row would cross a project boundary (HTTP 422).
    UnprocessableEntity(String),
    Internal(String),
}

impl ApiError {
    pub fn message(&self) -> &str {
        match self {
            ApiError::BadRequest(msg)
            | ApiError::Unauthorized(msg)
            | ApiError::NotFound(msg)
            | ApiError::Forbidden(msg)
            | ApiError::Conflict(msg)
            | ApiError::UnprocessableEntity(msg)
            | ApiError::Internal(msg) => msg,
        }
    }

    pub fn status(&self) -> Status {
        match self {
            ApiError::BadRequest(_) => Status::BadRequest,
            ApiError::Unauthorized(_) => Status::Unauthorized,
            ApiError::NotFound(_) => Status::NotFound,
            ApiError::Forbidden(_) => Status::Forbidden,
            ApiError::Conflict(_) => Status::Conflict,
            ApiError::UnprocessableEntity(_) => Status::UnprocessableEntity,
            ApiError::Internal(_) => Status::InternalServerError,
        }
    }
}

impl<'r> Responder<'r, 'static> for ApiError {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'static> {
        let status = self.status();
        Response::build_from(
            json!({
                "status": status.code,
                "error": status.reason_lossy(),
                "message": self.message(),
            })
            .respond_to(req)?,
        )
        .status(status)
        .ok()
    }
}

impl From<RepoError> for ApiError {
    fn from(value: RepoError) -> Self {
        match value {
            RepoError::NotFound => ApiError::NotFound("record not found".into()),
            RepoError::Db(DieselError::DatabaseError(
                DatabaseErrorKind::UniqueViolation
                | DatabaseErrorKind::ForeignKeyViolation
                | DatabaseErrorKind::CheckViolation
                | DatabaseErrorKind::NotNullViolation,
                info,
            )) => ApiError::BadRequest(info.message().to_string()),
            RepoError::Db(DieselError::NotFound) => ApiError::NotFound("record not found".into()),
            RepoError::Db(_) => ApiError::Internal("database query failed".into()),
            RepoError::Pool(err) => ApiError::Internal(format!("connection pool error: {}", err)),
            RepoError::BadInput(msg) => ApiError::BadRequest(msg),
            RepoError::Unauthorized => ApiError::Forbidden("operation not permitted".into()),
            RepoError::Duplicate(msg) => ApiError::Conflict(msg),
            RepoError::CrossProjectViolation(msg) => ApiError::UnprocessableEntity(msg),
        }
    }
}

impl From<Box<dyn std::error::Error>> for ApiError {
    fn from(value: Box<dyn std::error::Error>) -> Self {
        ApiError::Internal(value.to_string())
    }
}
