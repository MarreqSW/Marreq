// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! marreq-cloud library: hosted SaaS Marreq deployment modules.
//!
//! The library exposes the cloud-only API handlers, fairings, services, and
//! deployment-mode definition so that integration tests in
//! `marreq-cloud/tests/` can mount them via Rocket's local client. The
//! `marreq-cloud` binary in `src/main.rs` is a thin launcher over these
//! modules.

pub mod api;
pub mod deployment;
pub mod fairings;
pub mod routes;
pub mod services;
