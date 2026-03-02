// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Rocket fairings for the Marreq application.
//!
//! Fairings are middleware-like components that execute at various stages
//! of the Rocket lifecycle.

pub mod semantic_index;

pub use semantic_index::SemanticIndexFairing;
