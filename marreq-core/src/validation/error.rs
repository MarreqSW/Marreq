// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Field-level validation error type.
//!
//! Lives next to the validation helpers; bridges into the canonical
//! [`crate::api::error::ApiError`] via `From` so handlers can return validation
//! failures with `?`.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Field '{field}' is required")]
    Required { field: String },

    #[error("Field '{field}' is too long (max {max} characters)")]
    TooLong { field: String, max: usize },

    #[error("Field '{field}' is too short (min {min} characters)")]
    TooShort { field: String, min: usize },

    #[error("Invalid format for field '{field}': {message}")]
    InvalidFormat { field: String, message: String },

    #[error("Custom validation error: {0}")]
    Custom(String),
}
