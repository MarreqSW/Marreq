// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Build metadata and frontend↔backend compatibility ranges (issue #213).

use serde::Deserialize;
use serde_json::json;

/// Resolve the build override against the deployable crate's package version.
pub fn artifact_version(release: Option<&'static str>, package: &'static str) -> &'static str {
    release
        .filter(|version| !version.is_empty())
        .map(|version| version.strip_prefix('v').unwrap_or(version))
        .unwrap_or(package)
}

pub fn backend_version() -> &'static str {
    crate::deployment::try_current().map_or("dev", |mode| mode.artifact_version())
}

pub fn backend_git_sha() -> &'static str {
    crate::deployment::try_current().map_or("unknown", |mode| mode.source_sha())
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
        "backend_version": backend_version(),
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
        assert_eq!(c.max_version, "0.2.99");
    }

    #[test]
    fn artifact_version_prefers_the_release_override() {
        assert_eq!(artifact_version(Some("v0.2.3"), "0.2.1"), "0.2.3");
        assert_eq!(artifact_version(Some(""), "0.2.1"), "0.2.1");
        assert_eq!(artifact_version(None, "0.2.3"), "0.2.3");
    }

    #[test]
    fn build_info_preserves_legacy_fields() {
        let v = build_info_json("server");
        assert_eq!(v["backend_version"], backend_version());
        assert_eq!(v["deployment_mode"], "server");
        assert_eq!(v["frontend_compatibility"]["min_version"], "0.1.0");
        assert!(v["backend_git_sha"].as_str().is_some());
    }
}
