// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Service layer for the Marreq application.
//!
//! This module provides business logic services that abstract database operations
//! and provide a clean interface for route handlers.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Loggable, Logger};
use crate::models::User;
use crate::repository::errors::RepoError;
use crate::repository::PooledConnectionWrapper;

/// Trait providing reusable audit logging helpers for services.
///
/// Implementors only need to supply access to the shared [`AppState`]; the
/// default methods handle connection acquisition, context creation, and
/// best-effort error handling so that individual services do not duplicate
/// the same boilerplate.
pub trait AuditLog {
    fn app_state(&self) -> &AppState<DieselCachedRepo>;

    fn audit_conn(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.app_state().repo_read().inner_repo().get_conn()
    }

    fn audit_created<T: serde::Serialize + Loggable>(&self, actor: &User, id: i32, entity: &T) {
        if let Ok(mut conn) = self.audit_conn() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "audit: failed to log {} creation {id}: {_err}",
                    T::entity_type()
                );
            }
        }
    }

    fn audit_updated<T: serde::Serialize + Loggable>(&self, actor: &User, before: &T, after: &T) {
        if let Ok(mut conn) = self.audit_conn() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::updated(conn.as_mut(), &ctx, before, after) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "audit: failed to log {} update {} -> {}: {_err}",
                    T::entity_type(),
                    before.id(),
                    after.id()
                );
            }
        }
    }

    fn audit_deleted<T: serde::Serialize + Loggable>(&self, actor: &User, entity: &T) {
        if let Ok(mut conn) = self.audit_conn() {
            let ctx = LogCtx::new(actor.id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "audit: failed to log {} deletion {}: {_err}",
                    T::entity_type(),
                    entity.id()
                );
            }
        }
    }
}

pub mod applicability_service;
pub mod base_service;
pub mod baseline_service;
pub mod cache_service;
pub mod category_service;
pub mod comment_service;
pub mod custom_field_service;
pub mod decorated_requirement_service;
pub mod decorated_test_service;
pub mod email_sender;
pub mod group_service;
pub mod log_service;
pub mod matrix_service;
pub mod notification_service;
pub mod project_service;
pub mod registration_service;
pub mod reqif_service;
pub mod requirement_analytics_service;
pub mod requirement_diff_service;
pub mod requirement_service;
pub mod semantic_search;
pub mod status_service;
pub mod user_service;
pub mod verification_service;

#[cfg(test)]
mod tests;

pub use applicability_service::*;
pub use base_service::*;
pub use baseline_service::*;
pub use cache_service::*;
pub use category_service::*;
pub use comment_service::*;
pub use custom_field_service::*;
pub use decorated_requirement_service::*;
pub use decorated_test_service::*;
pub use group_service::*;
pub use log_service::*;
pub use matrix_service::*;
pub use notification_service::*;
pub use project_service::*;
pub use reqif_service::*;
pub use requirement_analytics_service::*;
pub use requirement_diff_service::*;
pub use requirement_service::*;
pub use semantic_search::*;
pub use status_service::*;
pub use user_service::*;
pub use verification_service::*;
