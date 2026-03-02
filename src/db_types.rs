// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

//! Custom PostgreSQL SQL type markers used by Diesel's generated schema.
//!
//! `pgvector::sql_types::Vector` is provided by the pgvector crate's diesel feature.
//! `Tsvector` is defined here because it has no upstream crate equivalent.

/// SQL type marker for PostgreSQL's built-in `tsvector` type.
#[derive(diesel::sql_types::SqlType)]
#[diesel(postgres_type(name = "tsvector", schema = "pg_catalog"))]
pub struct Tsvector;
