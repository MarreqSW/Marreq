// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

//! Data models for the requirement management application.
//!
//! These structures describe the core entities stored in the database and the
//! auxiliary forms used to create or update them. Most of the types derive
//! Diesel traits so they can be mapped directly to the PostgreSQL database.
//!
//! The models are organized into four modules:
//! - [`entities`]: Core database models that directly map to tables
//! - [`forms`]: Form structures for creating and updating entities
//! - [`decorators`]: Enriched models with human-readable values for presentation
//! - [`semantic_search`]: Models for RAG-powered semantic search

pub mod decorators;
pub mod entities;
pub mod forms;
pub mod semantic_search;

#[cfg(test)]
mod tests;

// Re-export all public types for backward compatibility
pub use decorators::*;
pub use entities::*;
pub use forms::*;
pub use semantic_search::*;
