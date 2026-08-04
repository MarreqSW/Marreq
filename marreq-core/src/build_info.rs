// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Build metadata and frontend↔backend compatibility ranges (issue #213).

use serde::Deserialize;
use serde_json::json;

/// Backend package version from `marreq-core/Cargo.toml`.
pub const BACKEND_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Git SHA injected at build time (`MARREQ_GIT_SHA`), or `"unknown"`.
pub fn backend_git_sha() -> &'static str {
    option_env!("MARREQ_GIT_SHA").unwrap_or("unknown")
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct FrontendCompatibility {
    pub min_version: String,
    pub max_version: String,
}

fn frontend_compatibility() -> FrontendCompatibility {
    serde_json::from_str(include_str!("../frontend_compatibility.json"))
        .expect("frontend_compatibility.json must be valid JSON")
}

/// JSON body for `GET /api/meta/build`.
pub fn build_info_json(deployment_mode: &str) -> serde_json::Value {
    let compat = frontend_compatibility();
    json!({
        "backend_version": BACKEND_VERSION,
        "backend_git_sha": backend_git_sha(),
        "deployment_mode": deployment_mode,
        "frontend_compatibility": {
            "min_version": compat.min_version,
            "max_version": compat.max_version,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatibility_json_loads() {
        let c = frontend_compatibility();
        assert_eq!(c.min_version, "0.1.0");
        assert_eq!(c.max_version, "0.1.99");
    }

    #[test]
    fn build_info_includes_cargo_version() {
        let v = build_info_json("server");
        assert_eq!(v["backend_version"], BACKEND_VERSION);
        assert_eq!(v["deployment_mode"], "server");
        assert_eq!(v["frontend_compatibility"]["min_version"], "0.1.0");
        assert!(v["backend_git_sha"].as_str().is_some());
    }
}
