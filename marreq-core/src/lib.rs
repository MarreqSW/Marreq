// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Marreq shared core library.
//!
//! Provides domain models, persistence, authentication primitives, and shared
//! Rocket routes/fairings consumed by both `marreq-server` and `marreq-cloud`.

#[macro_use]
extern crate rocket;
extern crate diesel;

pub mod api;
pub mod app;
pub mod auth;
pub mod authorization;
pub mod config;
pub mod cors;
pub mod db_types;
pub mod deployment;
pub mod diff;
pub mod fairings;
pub mod generators;
pub mod helper_functions;
pub mod importers;
pub mod logger;
pub mod models;
pub mod namespaces;
pub mod permissions;
pub mod repository;
pub mod reqif;
pub mod routes;
pub mod schema;
pub mod services;
pub mod status_enums;
pub mod validation;
