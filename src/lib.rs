// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Marreq - Requirements Management System
//!
//! This library provides the core functionality for the Marreq application.
//! It can be used both as a library (for testing) and as a binary application.

#[macro_use]
extern crate rocket;
extern crate diesel;

pub mod api;
pub mod app;
pub mod auth;
pub mod config;
pub mod db_types;
pub mod diff;
pub mod errors;
pub mod fairings;
pub mod generators;
pub mod helper_functions;
pub mod html;
pub mod importers;
pub mod logger;
pub mod models;
pub mod repository;
pub mod reqif;
pub mod routes;
pub mod schema;
pub mod services;
pub mod status_enums;
pub mod validation;
