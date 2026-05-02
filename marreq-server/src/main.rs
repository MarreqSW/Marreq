// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! marreq-server: self-hosted Marreq deployment binary.

pub mod api;
mod deployment;
mod routes;

#[rocket::main]
#[allow(clippy::result_large_err)]
async fn main() -> Result<(), rocket::Error> {
    marreq_core::config::AppConfig::install_from_env_or_exit();
    let mode: &'static dyn marreq_core::deployment::DeploymentMode = &deployment::INSTANCE;
    marreq_core::app::build_with(mode, routes::routes(), routes::fairings())
        .launch()
        .await?;
    Ok(())
}
