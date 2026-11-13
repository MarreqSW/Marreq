// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Self-hosted (admin-managed) deployment mode.

use marreq_core::deployment::DeploymentMode;

/// Self-hosted Marreq Server: admin-managed, no public registration.
pub struct Server;

impl DeploymentMode for Server {
    fn name(&self) -> &'static str {
        "server"
    }

    fn artifact_version(&self) -> &'static str {
        marreq_core::build_info::artifact_version(
            option_env!("MARREQ_VERSION"),
            env!("CARGO_PKG_VERSION"),
        )
    }

    fn source_sha(&self) -> &'static str {
        option_env!("MARREQ_GIT_SHA").unwrap_or("unknown")
    }
    fn allows_self_registration(&self) -> bool {
        false
    }
    fn requires_email_verification(&self) -> bool {
        false
    }
    fn allows_admin_promotion(&self) -> bool {
        true
    }
    fn assigns_personal_workspace(&self) -> bool {
        false
    }
}

pub static INSTANCE: Server = Server;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn server_mode_is_admin_managed() {
        let mode: &dyn DeploymentMode = &INSTANCE;
        assert_eq!(mode.name(), "server");
        assert!(!mode.allows_self_registration());
        assert!(!mode.requires_email_verification());
        assert!(mode.allows_admin_promotion());
        assert!(!mode.assigns_personal_workspace());
    }
}
