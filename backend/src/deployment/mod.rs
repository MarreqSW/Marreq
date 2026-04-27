// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Deployment-mode abstraction.
//!
//! Marreq is built as one of two mutually exclusive deployment modes:
//!
//! - **Server** (default, `--features server`): self-hosted, admin-managed.
//!   Public registration is disabled; an administrator creates accounts.
//! - **Cloud** (`--no-default-features --features cloud`): hosted SaaS.
//!   Anyone may self-register (with email verification); the single site
//!   administrator is bootstrapped from environment variables and cannot be
//!   granted via the API or UI.
//!
//! Behavioral differences are expressed through the [`DeploymentMode`] trait
//! so that services and routes can ask declarative questions
//! (`allows_self_registration()` etc.) instead of scattering `cfg!` checks.

#[cfg(all(feature = "server", feature = "cloud"))]
compile_error!(
    "features `server` and `cloud` are mutually exclusive; enable exactly one (default is `server`)"
);

#[cfg(not(any(feature = "server", feature = "cloud")))]
compile_error!("no deployment mode selected; enable exactly one of `server` (default) or `cloud`");

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

#[cfg(feature = "server")]
mod server_mode {
    use super::DeploymentMode;

    /// Self-hosted Marreq Server: admin-managed, no public registration.
    pub struct Server;

    impl DeploymentMode for Server {
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

    pub static INSTANCE: Server = Server;
}

#[cfg(feature = "cloud")]
mod cloud_mode {
    use super::DeploymentMode;

    /// Hosted Marreq Cloud: public registration with email verification, single
    /// env-bootstrapped site admin, personal workspace per user.
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
}

/// Returns the deployment mode chosen at compile time.
///
/// This is a zero-cost reference into a `static` impl; callers can hold it for
/// as long as they need.
pub fn current() -> &'static dyn DeploymentMode {
    #[cfg(feature = "server")]
    {
        &server_mode::INSTANCE
    }
    #[cfg(feature = "cloud")]
    {
        &cloud_mode::INSTANCE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_returns_a_known_mode() {
        let mode = current();
        assert!(matches!(mode.name(), "server" | "cloud"));
    }

    #[cfg(feature = "server")]
    #[test]
    fn server_mode_is_admin_managed() {
        let mode = current();
        assert_eq!(mode.name(), "server");
        assert!(!mode.allows_self_registration());
        assert!(!mode.requires_email_verification());
        assert!(mode.allows_admin_promotion());
        assert!(!mode.assigns_personal_workspace());
    }

    #[cfg(feature = "cloud")]
    #[test]
    fn cloud_mode_is_self_service() {
        let mode = current();
        assert_eq!(mode.name(), "cloud");
        assert!(mode.allows_self_registration());
        assert!(mode.requires_email_verification());
        assert!(!mode.allows_admin_promotion());
        assert!(mode.assigns_personal_workspace());
    }
}
