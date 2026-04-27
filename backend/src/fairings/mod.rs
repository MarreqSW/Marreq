// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Rocket fairings for the Marreq application.
//!
//! Fairings are middleware-like components that execute at various stages
//! of the Rocket lifecycle.

pub mod cache_control;
#[cfg(feature = "cloud")]
pub mod cloud_admin_bootstrap;
pub mod csrf;
pub mod security_headers;
pub mod semantic_index;

pub use cache_control::AntiCacheFairing;
#[cfg(feature = "cloud")]
pub use cloud_admin_bootstrap::CloudAdminBootstrapFairing;
pub use csrf::{csrf_denied, CsrfFairing};
pub use security_headers::SecurityHeadersFairing;
pub use semantic_index::SemanticIndexFairing;
