// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

//! Application configuration from environment.
//!
//! Reads optional settings; most behavior uses defaults when env vars are unset.

use std::sync::OnceLock;

static LOCK_APPROVED_VERSION_COMMENTS: OnceLock<bool> = OnceLock::new();

/// When true (default), POST comment with a requirement_version_id that is approved
/// returns 403. When false, approved versions can still receive comments.
pub fn lock_approved_version_comments() -> bool {
    *LOCK_APPROVED_VERSION_COMMENTS.get_or_init(|| {
        std::env::var("LOCK_APPROVED_VERSION_COMMENTS")
            .ok()
            .and_then(|s| match s.to_lowercase().as_str() {
                "1" | "true" | "yes" => Some(true),
                "0" | "false" | "no" => Some(false),
                _ => None,
            })
            .unwrap_or(true)
    })
}
