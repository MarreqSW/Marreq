// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Test-only deployment-mode helper.
//!
//! Exposed when the crate is compiled for tests or with the `test-helpers`
//! Cargo feature. Integration tests in `marreq-core/tests/` (and downstream
//! tests in the binary crates that consume `test-helpers`) call
//! [`install_test_server_mode`] from their fixtures so that
//! [`crate::deployment::current()`] returns a mode equivalent to the
//! self-hosted `marreq-server` binary without the production binaries having
//! to expose their concrete mode types.

use super::{set_current, DeploymentMode};

/// `Server`-equivalent deployment mode used by integration tests.
pub struct TestServerMode;

impl DeploymentMode for TestServerMode {
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

pub static TEST_SERVER: TestServerMode = TestServerMode;

/// Register [`TEST_SERVER`] as the active deployment mode.
///
/// Safe to call from many tests in parallel: the underlying `OnceLock`
/// silently ignores subsequent registrations after the first.
pub fn install_test_server_mode() {
    set_current(&TEST_SERVER);
}
