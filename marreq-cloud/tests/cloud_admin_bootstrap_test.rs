// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Integration test for [`marreq_cloud::fairings::cloud_admin_bootstrap::CloudAdminBootstrapFairing`].
//!
//! The fairing is attached by `marreq_cloud::routes::fairings()` and runs at
//! Rocket ignite. Given `MARREQ_SITE_ADMIN_EMAIL` plus
//! `MARREQ_SITE_ADMIN_BOOTSTRAP_PASSWORD`, it must create the configured
//! site administrator with `is_admin = true` and `email_verified = true`.

mod common;

use common::cloud_client;
use marreq_core::repository::UserRepository;

#[rocket::async_test]
async fn bootstraps_site_admin_user_on_ignite() {
    // The fairing reads these on `on_ignite`, so they must be set before
    // building the Rocket instance.
    std::env::set_var("MARREQ_SITE_ADMIN_EMAIL", "root@example.com");
    std::env::set_var(
        "MARREQ_SITE_ADMIN_BOOTSTRAP_PASSWORD",
        "RootBootstrap!Pass_2026",
    );

    let client = cloud_client().await;

    let state = common::app_state(&client);
    let repo = state.repo_read();
    let admin = repo
        .get_user_by_email("root@example.com")
        .expect("repo lookup")
        .expect("bootstrap fairing should have created the site admin");

    assert!(admin.is_admin, "bootstrapped admin must have is_admin=true");
    assert!(
        admin.email_verified,
        "bootstrapped admin must have email_verified=true"
    );
    assert_eq!(admin.email, "root@example.com");
}
