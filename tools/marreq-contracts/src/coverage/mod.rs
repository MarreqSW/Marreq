//! Conservative TypeScript call analysis using the real TS/TSX grammar.
//! Unknown URL or HTTP-method expressions fail instead of dropping dependencies.
use crate::contract::{Operation, operations};
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Debug, Serialize)]
pub struct Coverage {
    pub operations: BTreeSet<Operation>,
    pub call_sites: usize,
}

mod analyzer;
mod routes;
mod syntax;
mod transports;
pub use routes::{intersects, route_pattern};

/// Analyze one source text; `scan` supplies audited transport boundaries.
pub fn analyze(
    source: &str,
    tsx: bool,
    declared: &BTreeMap<Operation, Value>,
    transport: bool,
) -> Result<Coverage> {
    analyzer::analyze_file(source, tsx, declared, transport, None)
}

pub fn scan(root: &Path, component: &str, spec: &Value) -> Result<Coverage> {
    anyhow::ensure!(
        matches!(component, "frontend" | "mcp"),
        "consumer must be frontend or mcp"
    );
    let directory = root.join(if component == "frontend" {
        "frontend/src"
    } else {
        "mcp-server/src"
    });
    let declared = operations(spec, None)?;
    let mut result = Coverage {
        operations: BTreeSet::new(),
        call_sites: 0,
    };
    for entry in walkdir::WalkDir::new(directory) {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type().is_file()
            || !matches!(
                path.extension().and_then(|x| x.to_str()),
                Some("ts" | "tsx")
            )
        {
            continue;
        }
        let relative = path.strip_prefix(root)?.to_string_lossy();
        if relative.contains("/__tests__/")
            || relative.ends_with(".test.ts")
            || relative.ends_with("generated-contract.ts")
        {
            continue;
        }
        let source = std::fs::read_to_string(path)?;
        let transport =
            relative == "frontend/src/api/transport.ts" || relative == "mcp-server/src/client.ts";
        let coverage = analyzer::analyze_file(
            &source,
            path.extension().is_some_and(|x| x == "tsx"),
            &declared,
            transport,
            Some(relative.as_ref()),
        )
        .with_context(|| relative.to_string())?;
        result.call_sites += coverage.call_sites;
        result.operations.extend(coverage.operations);
    }
    // Protocol navigation/discovery isn't fetch(): explicit, small, tested roots.
    let protocol = if component == "frontend" {
        vec!["auth_auth_external_start", "auth_auth_external_callback"]
    } else {
        vec![
            "oauth_authorization_server_metadata",
            "oauth_protected_resource_metadata_mcp",
            "oauth_register",
            "oauth_authorize_page",
            "oauth_authorize_decision",
            "oauth_token",
        ]
    };
    for id in protocol {
        let key = declared
            .iter()
            .find(|(_, v)| v["operationId"] == id)
            .map(|(k, _)| k.clone())
            .with_context(|| format!("missing protocol operation {id}"))?;
        result.operations.insert(key);
    }
    Ok(result)
}

/// Verify every discovered dependency and report each consumer's coverage.
pub fn validate(root: &Path, spec: &Value) -> Result<serde_json::Map<String, Value>> {
    let mut counts = serde_json::Map::new();
    for component in ["frontend", "mcp"] {
        let coverage = scan(root, component, spec)?;
        let selected = crate::contract::select(spec, &coverage.operations, None)?;
        crate::contract::validate_wire_contract(&selected)?;
        counts.insert(
            component.into(),
            serde_json::json!(coverage.operations.len()),
        );
    }
    Ok(counts)
}
