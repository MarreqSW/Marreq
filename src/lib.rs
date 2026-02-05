//! ReqMan - Requirements Management System
//!
//! This library provides the core functionality for the ReqMan application.
//! It can be used both as a library (for testing) and as a binary application.

#[macro_use]
extern crate rocket;
extern crate diesel;

pub mod api;
pub mod app;
pub mod auth;
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
