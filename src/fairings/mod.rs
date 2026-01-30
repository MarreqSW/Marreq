//! Rocket fairings for the ReqMan application.
//!
//! Fairings are middleware-like components that execute at various stages
//! of the Rocket lifecycle.

pub mod semantic_index;

pub use semantic_index::SemanticIndexFairing;
