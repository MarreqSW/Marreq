// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid username or password")]
    InvalidCredentials, // user not found OR bad password

    #[error("not logged in")]
    NotLoggedIn, // session cookie missing

    #[error("invalid session")]
    InvalidSession, // session cookie couldn't be parsed

    #[error("database error: {0}")]
    Db(String), // repo errors, connection pool, etc.

    #[error("password verification error: {0}")]
    Verify(String), // hashing library errors

    #[error("logging error: {0}")]
    Audit(String), // optional: login logging failed

    #[error(transparent)]
    Repo(#[from] crate::repository::errors::RepoError),
}
