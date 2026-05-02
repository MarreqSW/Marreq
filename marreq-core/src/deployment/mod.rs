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
/// has registered a mode.  Library code that may run before installation should
/// prefer [`try_current`].
pub fn current() -> &'static dyn DeploymentMode {
    *CURRENT
        .get()
        .expect("deployment::current() called before app::build_with set the mode")
}

/// Non-panicking variant of [`current`]. Returns `None` when no deployment mode
/// has been installed yet — useful in code paths that may be exercised from
/// startup hooks, tests, or shared library helpers that pre-date Rocket boot.
pub fn try_current() -> Option<&'static dyn DeploymentMode> {
    CURRENT.get().copied()
}

/// True once a deployment mode has been registered via [`set_current`].
pub fn is_initialized() -> bool {
    CURRENT.get().is_some()
}

#[cfg(any(test, feature = "test-helpers"))]
pub mod test_support;
#[cfg(any(test, feature = "test-helpers"))]
pub use test_support::install_test_server_mode;
