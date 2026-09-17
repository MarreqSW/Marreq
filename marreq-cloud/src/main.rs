// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! marreq-cloud: hosted SaaS Marreq deployment binary.

#[rocket::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), rocket::Error> {
    marreq_core::config::AppConfig::install_from_env_or_exit();
    let mode: &'static dyn marreq_core::deployment::DeploymentMode =
        &marreq_cloud::deployment::INSTANCE;
    let auth = marreq_core::auth::AuthConfig::cloud_from_env().unwrap_or_else(|error| {
        eprintln!("Invalid authentication configuration: {error}");
        std::process::exit(2);
    });
    marreq_core::app::build_with_auth(
        mode,
        auth,
        marreq_cloud::routes::routes(),
        marreq_cloud::routes::fairings(),
    )
    .launch()
    .await?;
    Ok(())
}
