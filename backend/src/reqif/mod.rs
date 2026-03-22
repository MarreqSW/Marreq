// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! ReqIF 1.2 import and export support.
//!
//! See [OMG ReqIF 1.2](https://www.omg.org/spec/ReqIF/1.2/).

pub mod export;
pub mod import;
pub mod mapping;
pub mod schema;

pub use export::to_reqif;
pub use import::{parse_reqif, ImportConfig, ImportResult, ParsedDocument};
pub use mapping::default_attribute_mapping;
pub use schema::{ParsedSpecObject, ParsedSpecRelation};
