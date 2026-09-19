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
    /// Optional LONG-NAME on the SPEC-OBJECT element.
    pub long_name: Option<String>,
    /// LAST-CHANGE if present (not mapped into Marreq).
    pub last_change: Option<String>,
    /// Attribute long-name or identifier -> value (string).
    pub attributes: HashMap<String, String>,
}

/// A relation between two SpecObjects (e.g. parent-child or trace).
#[derive(Debug, Clone)]
pub struct ParsedSpecRelation {
    pub id: String,
    pub type_ref: String,
    /// Source SpecObject identifier (e.g. child).
    pub source: String,
    /// Target SpecObject identifier (e.g. parent).
    pub target: String,
}

/// One parent/child edge taken from SPEC-HIERARCHY (child, parent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedHierarchyEdge {
    pub child_id: String,
    pub parent_id: String,
}
