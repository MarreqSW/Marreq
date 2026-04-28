// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use rocket::fairing::Fairing;
use rocket::Route;
use std::sync::Arc;

use crate::api::auth_public;
use crate::fairings::cloud_admin_bootstrap::CloudAdminBootstrapFairing;

/// Cloud-only Rocket routes (public auth endpoints).
pub fn routes() -> Vec<Route> {
    rocket::routes![
        auth_public::auth_register,
        auth_public::auth_verify_email,
        auth_public::auth_forgot_password,
        auth_public::auth_reset_password,
    ]
}

/// Cloud-only Rocket fairings.
pub fn fairings() -> Vec<Arc<dyn Fairing>> {
    vec![Arc::new(CloudAdminBootstrapFairing) as Arc<dyn Fairing>]
}
