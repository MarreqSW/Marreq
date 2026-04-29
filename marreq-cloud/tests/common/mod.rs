// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Shared test fixtures for `marreq-cloud` integration tests.
//!
//! Builds a Rocket instance through the production
//! [`marreq_core::app::build_with`] pipeline using the `Cloud` deployment
//! mode plus the cloud-only routes and fairings, then wraps it in a tracked
//! local Rocket client. With `marreq-core`'s `test-helpers` feature
//! enabled (declared in `marreq-cloud/Cargo.toml`'s dev-dependencies),
//! `build_with` constructs an in-memory `DieselRepoMock`-backed
//! `AppState<CacheRepository<DieselRepoMock>>` internally — no real
//! database is required.

use marreq_core::app::{AppState, DieselCachedRepo};
use rocket::local::asynchronous::Client;

pub type TestAppState = AppState<DieselCachedRepo>;

/// Build a Rocket instance for the cloud deployment and wrap it in a
/// tracked local client.
pub async fn cloud_client() -> Client {
    let rocket = marreq_core::app::build_with(
        &marreq_cloud::deployment::INSTANCE,
        marreq_cloud::routes::routes(),
        marreq_cloud::routes::fairings(),
    );
    Client::tracked(rocket)
        .await
        .expect("rocket should ignite for cloud integration tests")
}

/// Convenience accessor for the managed `AppState` on a built client.
pub fn app_state(client: &Client) -> &TestAppState {
    client
        .rocket()
        .state::<TestAppState>()
        .expect("AppState should be managed by build_with")
}
