// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Deployment-mode abstraction.
//!
//! Behavioral differences between deployment variants (self-hosted server vs.
//! hosted cloud) are expressed through the [`DeploymentMode`] trait so that
//! services and routes can ask declarative questions (`allows_self_registration()`
//! etc.) instead of scattering `cfg!` checks.
//!
//! The concrete implementations live in the deployment-specific binary crates
//! (`marreq-server`, `marreq-cloud`). At startup each binary calls
//! [`app::build_with`] which registers the chosen impl via [`set_current`].

use std::sync::OnceLock;

/// Behavioral toggles that differ between deployment modes.
///
/// All implementations are pure / side-effect free; this trait is intended to
/// be queried freely from services, guards, and routes.
pub trait DeploymentMode: Send + Sync {
    /// Stable identifier for the deployment mode (e.g. `"server"` / `"cloud"`).
    fn name(&self) -> &'static str;

    /// True when unauthenticated visitors may create their own account through
    /// the public registration endpoint.
    fn allows_self_registration(&self) -> bool;

    /// True when newly created accounts must verify their email address before
    /// they can log in.
    fn requires_email_verification(&self) -> bool;

    /// True when an existing administrator may grant the `is_admin` flag to
    /// other users through the API or UI. In cloud mode the single site
    /// administrator is bootstrapped from the environment and no other user
    /// may be promoted.
    fn allows_admin_promotion(&self) -> bool;

    /// True when each newly created user should automatically be given a
    /// personal workspace (GitLab-style namespace) that owns their groups
    /// and projects.
    fn assigns_personal_workspace(&self) -> bool;

    /// True when an administrator may create a user account through the
    /// `POST /api/users` endpoint. False in cloud mode (users must self-register).
    fn allows_self_administered_user_creation(&self) -> bool {
        // By default this mirrors `allows_admin_promotion`; cloud mode opts out.
        self.allows_admin_promotion()
    }
}

static CURRENT: OnceLock<&'static dyn DeploymentMode> = OnceLock::new();

/// Register the active deployment mode.  Called exactly once by [`crate::app::build_with`].
/// Subsequent calls (e.g. from parallel tests setting the same mode) are silently ignored.
pub fn set_current(mode: &'static dyn DeploymentMode) {
    let _ = CURRENT.set(mode);
}

/// Returns the active deployment mode.
///
/// # Panics
/// Panics if called before [`crate::app::build_with`] (or a test's explicit [`set_current`] call)
/// has registered a mode.
pub fn current() -> &'static dyn DeploymentMode {
    *CURRENT
        .get()
        .expect("deployment::current() called before app::build_with set the mode")
}

/// Fallback mode used by the legacy `backend/` binary and in-tree unit tests
/// until the binary crates take over startup.  Mirrors the self-hosted Server
/// defaults.
///
/// # Note
/// This function will be deleted along with the `backend/` crate once the
/// `marreq-server` / `marreq-cloud` split is complete.
#[doc(hidden)]
pub fn default_mode() -> &'static dyn DeploymentMode {
    static DEFAULT: DefaultMode = DefaultMode;
    &DEFAULT
}

struct DefaultMode;

impl DeploymentMode for DefaultMode {
    fn name(&self) -> &'static str {
        "server"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_returns_a_known_mode() {
        let mode = default_mode();
        assert!(matches!(mode.name(), "server" | "cloud"));
    }
}
