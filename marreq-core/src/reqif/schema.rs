// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Minimal ReqIF 1.2 structures for in-memory representation (import).

use std::collections::HashMap;

/// One requirement-like artifact parsed from a ReqIF SpecObject.
#[derive(Debug, Clone)]
pub struct ParsedSpecObject {
    /// ReqIF identifier (e.g. UUID).
    pub id: String,
    /// Type identifier (e.g. reference to SpecObjectType).
    pub type_ref: String,
    /// Attribute long-name or identifier -> value (string).
    pub attributes: HashMap<String, String>,
}

/// A relation between two SpecObjects (e.g. parent-child).
#[derive(Debug, Clone)]
pub struct ParsedSpecRelation {
    pub id: String,
    pub type_ref: String,
    /// Source SpecObject identifier (e.g. child).
    pub source: String,
    /// Target SpecObject identifier (e.g. parent).
    pub target: String,
}
