//! Load editable OpenAPI sources through the pinned, offline Redocly bundler.
//! Retained artifact documents use `contract::load` and never run a resolver.
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{path::Path, process::Command};

pub const DEFAULT_SPEC: &str = "docs/developer/openapi/root.yaml";

pub fn load(path: &Path) -> Result<Value> {
    let output = Command::new("node")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/bundle-openapi.mjs"))
        .arg(path)
        .output()
        .context("OpenAPI bundling requires Node 22+ and `npm ci` at repository root")?;
    ensure!(
        output.status.success(),
        "OpenAPI bundling failed for {}: {}",
        path.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).context("invalid bundled OpenAPI JSON")
}
