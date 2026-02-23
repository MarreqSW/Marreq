// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ReqMan

//! Rocket fairings for the ReqMan application.
//!
//! Fairings are middleware-like components that execute at various stages
//! of the Rocket lifecycle.

pub mod semantic_index;

pub use semantic_index::SemanticIndexFairing;
