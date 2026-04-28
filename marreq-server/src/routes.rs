// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use rocket::fairing::Fairing;
use rocket::Route;
use std::sync::Arc;

/// Server-only Rocket routes (admin user management).
pub fn routes() -> Vec<Route> {
    Vec::new()
}

/// Server-only Rocket fairings.
pub fn fairings() -> Vec<Arc<dyn Fairing>> {
    Vec::new()
}
