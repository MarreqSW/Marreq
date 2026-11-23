use std::{collections::BTreeMap, fs, path::Path, sync::LazyLock};

use anyhow::{Context, Result, ensure};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::{
    Repository, json,
    registry::{BACKENDS, Component, Registry, StableVersion},
};

static CONTRACT_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\A[A-Za-z0-9_.-]+(?:/[A-Za-z0-9_.-]+)*\.json\z").expect("contract path regex")
});

pub(crate) type Records = BTreeMap<Component, Baseline>;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Baseline {
    pub version: StableVersion,
    pub source_sha: String,
    pub source_tag: String,
    pub image: String,
    pub digest: String,
    pub contract_sha256: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Manifest {
    pub format: String,
    pub variant: Component,
    pub product_release: Option<StableVersion>,
    pub components: BTreeMap<Component, Artifact>,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct Artifact {
    pub service: String,
    pub version: StableVersion,
    pub source_sha: String,
    pub source_tag: Option<String>,
    pub image: String,
    pub digest: String,
    pub contract: ContractReference,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContractReference {
    pub file: String,
    pub sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Contract {
    format: String,
    component: String,
    version: StableVersion,
    source_sha: String,
    #[serde(rename = "contract")]
    _operations: Map<String, Value>,
    variant: Option<Component>,
    #[serde(rename = "compatibility")]
    _compatibility: Option<Value>,
    #[serde(rename = "frontend_compatibility")]
    _frontend_compatibility: Option<Value>,
}

fn nonzero_hex(value: &str, size: usize) -> bool {
    value.len() == size
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        && value.bytes().any(|byte| byte != b'0')
}

pub(crate) fn full_sha(value: &str) -> bool {
    nonzero_hex(value, 40)
}

pub(crate) fn source_tag(component: Component, version: &StableVersion) -> String {
    format!("marreq-{component}-v{version}")
}

pub(crate) fn validate_contract(
    root: &Path,
    reference: &ContractReference,
    component: Component,
    version: &StableVersion,
    source_sha: &str,
) -> Result<()> {
    let name = &reference.file;
    ensure!(
        CONTRACT_PATH.is_match(name) && !name.split('/').any(|part| matches!(part, "." | "..")),
        "invalid contract path"
    );
    let path = root
        .join(name)
        .canonicalize()
        .context("missing retained contract")?;
    ensure!(
        path.starts_with(root) && path.is_file(),
        "missing retained contract"
    );
    let contents = fs::read(path)?;
    ensure!(
        hex::encode(Sha256::digest(&contents)) == reference.sha256,
        "contract checksum mismatch"
    );
    let bundle: Contract = json::decode(&contents)?;
    let (kind, variant) = if component.is_backend() {
        ("backend", Some(component))
    } else {
        (component.name(), None)
    };
    ensure!(
        bundle.format == "marreq.contract.v2"
            && bundle.component == kind
            && bundle.variant == variant
            && &bundle.version == version
            && bundle.source_sha == source_sha,
        "contract identity mismatch"
    );
    Ok(())
}

pub(crate) fn image(component: Component) -> String {
    let repository = match component {
        Component::Frontend => "marreq-frontend",
        Component::Mcp => "marreq-mcp-server",
        backend => backend.directory(),
    };
    format!("ghcr.io/marreqsw/{repository}")
}

pub(crate) fn digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| nonzero_hex(hex, 64))
}

pub(crate) fn services(variant: Component) -> [(Component, &'static str); 3] {
    use Component::{Cloud, Frontend, Mcp, Server};
    match variant {
        Server => [
            (Server, "marreq-server"),
            (Frontend, "frontend"),
            (Mcp, "mcp-server"),
        ],
        Cloud => [
            (Cloud, "marreq-cloud"),
            (Frontend, "frontend-cloud"),
            (Mcp, "mcp-cloud"),
        ],
        _ => unreachable!("only backend variants have provenance"),
    }
}

fn validate_manifest(path: &Path, variant: Component) -> Result<Manifest> {
    let manifest: Manifest = json::decode(&fs::read(path)?)?;
    ensure!(
        manifest.format == "marreq.deployment-provenance.v1",
        "unsupported provenance format"
    );
    ensure!(
        manifest.variant == variant,
        "expected {variant} provenance variant"
    );
    let services = services(variant);
    ensure!(
        manifest.components.len() == services.len(),
        "invalid object fields"
    );
    let resolved = path.canonicalize()?;
    let root = resolved
        .parent()
        .context("expected provenance parent directory")?;
    for (component, service) in services {
        let artifact = manifest
            .components
            .get(&component)
            .context("invalid object fields")?;
        ensure!(
            artifact.service == service,
            "{component}: invalid service mapping"
        );
        ensure!(
            full_sha(&artifact.source_sha),
            "{component}: invalid source SHA"
        );
        ensure!(
            artifact.image == image(component),
            "{component}: invalid image repository"
        );
        ensure!(
            digest(&artifact.digest),
            "{component}: invalid registry digest"
        );
        ensure!(
            artifact
                .source_tag
                .as_ref()
                .is_none_or(|tag| *tag == source_tag(component, &artifact.version)),
            "{component}: invalid source tag"
        );
        validate_contract(
            root,
            &artifact.contract,
            component,
            &artifact.version,
            &artifact.source_sha,
        )?;
    }
    Ok(manifest)
}

pub(crate) fn publications(
    repo: &Repository,
    paths: [Option<&Path>; 2],
    prior: &Registry,
    product: Option<&StableVersion>,
) -> Result<Records> {
    let mut records = Records::new();
    for (variant, path) in BACKENDS.into_iter().zip(paths) {
        let path = path.with_context(|| {
            format!("missing trusted {variant} publication provenance manifest")
        })?;
        let manifest = validate_manifest(path, variant)
            .with_context(|| format!("{variant} publication provenance"))?;
        ensure!(
            manifest.product_release.as_ref() == product,
            "{variant}: provenance must identify prior product release {}",
            product.map_or_else(|| "null".to_owned(), ToString::to_string)
        );
        for (component, record) in manifest.components {
            let floor = prior.component(component);
            let allowed = if product.is_none() {
                record.version >= *floor
            } else {
                record.version == *floor
            };
            ensure!(
                allowed,
                "{component}: publication {} conflicts with prior registry version {floor}",
                record.version
            );
            let tag = source_tag(component, &record.version);
            ensure!(
                record.source_tag.as_ref() == Some(&tag),
                "{component}: missing publication source tag {tag}"
            );
            ensure!(
                repo.resolve(&format!("refs/tags/{tag}"))? == record.source_sha,
                "{component}: {tag} disagrees with the retained publication SHA"
            );
            // Two variants can retain the same contract bytes under different filenames.
            let identity = Baseline {
                version: record.version,
                source_sha: record.source_sha,
                source_tag: tag,
                image: record.image,
                digest: record.digest,
                contract_sha256: record.contract.sha256,
            };
            if let Some(old) = records.insert(component, identity) {
                ensure!(
                    records[&component] == old,
                    "{component}: conflicting publication evidence across variants"
                );
            }
        }
    }
    Ok(records)
}
