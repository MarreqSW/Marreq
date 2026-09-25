// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use marreq_core::api::RoutePolicy;
use rocket::fairing::Fairing;
use rocket::Route;
use std::sync::Arc;

/// Server-only Rocket routes. Currently empty: admin user management lives in marreq-core and is gated by the deployment-mode trait. Placeholder for future server-only handlers.
pub fn routes() -> Vec<(RoutePolicy, Route)> {
    Vec::new()
}

/// Server-only Rocket fairings. Currently empty; placeholder for future server-only fairings.
pub fn fairings() -> Vec<Arc<dyn Fairing>> {
    Vec::new()
}
