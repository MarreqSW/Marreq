// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Hosted SaaS deployment mode: public registration with email verification,
//! single env-bootstrapped site administrator, personal workspace per user.

use marreq_core::deployment::DeploymentMode;

pub struct Cloud;

impl DeploymentMode for Cloud {
    fn name(&self) -> &'static str {
        "cloud"
    }
    fn allows_self_registration(&self) -> bool {
        true
    }
    fn requires_email_verification(&self) -> bool {
        true
    }
    fn allows_admin_promotion(&self) -> bool {
        false
    }
    fn assigns_personal_workspace(&self) -> bool {
        true
    }
}

pub static INSTANCE: Cloud = Cloud;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cloud_mode_is_self_service() {
        let mode: &dyn DeploymentMode = &INSTANCE;
        assert_eq!(mode.name(), "cloud");
        assert!(mode.allows_self_registration());
        assert!(mode.requires_email_verification());
        assert!(!mode.allows_admin_promotion());
        assert!(mode.assigns_personal_workspace());
    }
}
