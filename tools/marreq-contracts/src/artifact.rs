//! Marreq artifact identity/policy. Contract hashes pin files, NOT Docker images.
use crate::{contract, coverage};
use anyhow::{Context, Result, ensure};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub requires_backend_min: Version,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_backend_max: Option<Version>,
}
impl Policy {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.requires_backend_max
                .as_ref()
                .is_none_or(|max| max.cmp_precedence(&self.requires_backend_min).is_ge()),
            "inverted backend compatibility range"
        );
        Ok(())
    }
    pub fn accepts(&self, backend: &Version) -> Result<()> {
        self.validate()?;
        ensure!(
            backend.cmp_precedence(&self.requires_backend_min).is_ge()
                && self
                    .requires_backend_max
                    .as_ref()
                    .is_none_or(|max| backend.cmp_precedence(max).is_le()),
            "backend {backend} outside consumer policy"
        );
        Ok(())
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendRange {
    pub min_version: Version,
    pub max_version: Version,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub format: String,
    pub component: String,
    pub version: Version,
    pub source_sha: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<Policy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontend_compatibility: Option<FrontendRange>,
    pub contract: Value,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub file: PathBuf,
    pub sha256: String,
    pub version: Version,
    pub source_sha: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub format: String,
    pub variant: String,
    pub backend: Artifact,
    pub frontend: Artifact,
    pub mcp: Artifact,
}
pub fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn sha(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}
fn component(value: &str) -> Result<()> {
    ensure!(
        ["backend", "frontend", "mcp"].contains(&value),
        "invalid component {value}"
    );
    Ok(())
}
pub fn variant(value: &str) -> Result<()> {
    ensure!(
        ["server", "cloud"].contains(&value),
        "invalid deployment variant {value}"
    );
    Ok(())
}
fn identity(bundle: &Bundle) -> Result<()> {
    ensure!(
        bundle.format == "marreq.contract.v2",
        "unsupported bundle; historical consumer dependencies must be evidenced, not inferred from current main"
    );
    component(&bundle.component)?;
    ensure!(sha(&bundle.source_sha, 40), "immutable source SHA required");
    if bundle.component == "backend" {
        variant(
            bundle
                .variant
                .as_deref()
                .context("backend variant required")?,
        )?;
        ensure!(
            bundle.frontend_compatibility.is_some(),
            "backend frontend policy missing"
        );
    } else {
        ensure!(
            bundle.variant.is_none(),
            "consumer artifacts are deployment-independent"
        );
        bundle
            .compatibility
            .as_ref()
            .context("consumer compatibility policy missing")?
            .validate()?;
    }
    ensure!(
        !contract::operations(&bundle.contract, None)?.is_empty(),
        "empty artifact contract is missing evidence"
    );
    Ok(())
}
pub fn export(root: &Path, who: &str, version: Version, mode: Option<String>) -> Result<Bundle> {
    component(who)?;
    let spec = contract::load(&root.join("docs/developer/openapi.yaml"))?;
    let required: BTreeSet<_> = if who == "backend" {
        variant(mode.as_deref().context("backend --variant required")?)?;
        contract::operations(&spec, mode.as_deref())?
            .into_iter()
            .filter(|(_, op)| {
                !op.get("x-marreq-unverified")
                    .is_some_and(|v| v != &Value::Bool(false))
            })
            .map(|(key, _)| key)
            .collect()
    } else {
        coverage::scan(root, who, &spec)?.operations
    };
    let selected = contract::select(&spec, &required, mode.as_deref(), true)?;
    if who == "backend" {
        contract::validate(&selected)?;
    } else {
        contract::validate_wire_contract(&selected)?;
    }
    let directory = if who == "mcp" { "mcp-server" } else { who };
    let mut inputs: Vec<String> = [
        "docs/developer/openapi.yaml",
        "tools/marreq-contracts",
        "Cargo.toml",
        "Cargo.lock",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    if who == "backend" {
        inputs.extend(
            ["marreq-core", "marreq-cloud", "marreq-server"]
                .into_iter()
                .map(String::from),
        );
    } else {
        inputs.extend(
            [
                "src",
                "package.json",
                "package-lock.json",
                "compatibility.json",
            ]
            .into_iter()
            .map(|path| format!("{directory}/{path}")),
        );
    }
    let status = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all", "--"])
        .args(&inputs)
        .current_dir(root)
        .output()?;
    ensure!(
        status.status.success() && status.stdout.is_empty(),
        "commit source/contract inputs before exporting an immutable bundle"
    );
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()?;
    ensure!(output.status.success(), "cannot determine source identity");
    let source_sha = String::from_utf8(output.stdout)?.trim().to_owned();
    let policy = if who != "backend" {
        Some(serde_json::from_value(contract::load(
            &root.join(directory).join("compatibility.json"),
        )?)?)
    } else {
        None
    };
    let frontend_compatibility = if who == "backend" {
        Some(serde_json::from_value(contract::load(
            &root.join("marreq-core/frontend_compatibility.json"),
        )?)?)
    } else {
        None
    };
    let bundle = Bundle {
        format: "marreq.contract.v2".into(),
        component: who.into(),
        version,
        source_sha,
        variant: mode,
        compatibility: policy,
        frontend_compatibility,
        contract: selected,
    };
    identity(&bundle)?;
    Ok(bundle)
}
fn read(directory: &Path, descriptor: &Artifact, who: &str) -> Result<Bundle> {
    ensure!(
        !descriptor.file.is_absolute()
            && !descriptor
                .file
                .components()
                .any(|x| matches!(x, std::path::Component::ParentDir)),
        "artifact file must stay inside plan directory"
    );
    let path = directory.join(&descriptor.file);
    let canonical = path.canonicalize().with_context(|| {
        format!(
            "missing historical {who} artifact {}; refusing substitution with main",
            path.display()
        )
    })?;
    ensure!(
        canonical.starts_with(directory.canonicalize()?),
        "artifact symlink escapes plan directory"
    );
    let bytes = std::fs::read(canonical)?;
    ensure!(
        sha(&descriptor.sha256, 64) && hash(&bytes) == descriptor.sha256,
        "{who}: contract SHA-256 mismatch (not an image digest)"
    );
    let bundle: Bundle =
        serde_json::from_slice(&bytes).with_context(|| format!("invalid {who} bundle"))?;
    identity(&bundle)?;
    ensure!(
        bundle.component == who
            && bundle.version == descriptor.version
            && bundle.source_sha == descriptor.source_sha,
        "{who}: artifact component/version/source SHA mismatch"
    );
    contract::validate(&bundle.contract)?;
    Ok(bundle)
}
pub fn verify(path: &Path, candidate: Option<(&str, &Version, &str, &Path)>) -> Result<Value> {
    let plan: Plan = serde_json::from_value(contract::load(path)?)?;
    ensure!(
        plan.format == "marreq.deployment-contract.v2",
        "unsupported deployment plan format"
    );
    variant(&plan.variant)?;
    let directory = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let backend = read(directory, &plan.backend, "backend")?;
    let frontend = read(directory, &plan.frontend, "frontend")?;
    let mcp = read(directory, &plan.mcp, "mcp")?;
    ensure!(
        backend.variant.as_deref() == Some(&plan.variant),
        "backend deployment variant mismatch"
    );
    if let Some((who, ver, source, file)) = candidate {
        let descriptor = match who {
            "backend" => &plan.backend,
            "frontend" => &plan.frontend,
            "mcp" => &plan.mcp,
            _ => anyhow::bail!("invalid candidate component"),
        };
        ensure!(
            &descriptor.version == ver
                && descriptor.source_sha == source
                && hash(&std::fs::read(file)?) == descriptor.sha256,
            "plan does not reference the source-exported tagged candidate"
        );
    }
    let range = backend.frontend_compatibility.as_ref().unwrap();
    ensure!(
        range.min_version.cmp_precedence(&range.max_version).is_le(),
        "invalid backend frontend range"
    );
    ensure!(
        frontend.version.cmp_precedence(&range.min_version).is_ge()
            && frontend.version.cmp_precedence(&range.max_version).is_le(),
        "frontend version outside backend policy"
    );
    for consumer in [&frontend, &mcp] {
        consumer
            .compatibility
            .as_ref()
            .unwrap()
            .accepts(&backend.version)
            .with_context(|| consumer.component.clone())?;
        contract::compare(&consumer.contract, &backend.contract, &plan.variant)
            .with_context(|| consumer.component.clone())?;
    }
    Ok(
        serde_json::json!({"compatible":true,"variant":plan.variant,"versions":{"backend":backend.version,"frontend":frontend.version,"mcp":mcp.version}}),
    )
}
