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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: String,
    variant: Component,
    product_release: Option<StableVersion>,
    components: BTreeMap<Component, Artifact>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    service: String,
    version: StableVersion,
    source_sha: String,
    source_tag: Option<String>,
    image: String,
    digest: String,
    contract: ContractReference,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractReference {
    file: String,
    sha256: String,
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

fn validate_contract(root: &Path, artifact: &Artifact, component: Component) -> Result<()> {
    let name = &artifact.contract.file;
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
        hex::encode(Sha256::digest(&contents)) == artifact.contract.sha256,
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
            && bundle.version == artifact.version
            && bundle.source_sha == artifact.source_sha,
        "contract identity mismatch"
    );
    Ok(())
}

fn validate_manifest(path: &Path, variant: Component) -> Result<Manifest> {
    use Component::{Cloud, Frontend, Mcp, Server};
    let manifest: Manifest = json::decode(&fs::read(path)?)?;
    ensure!(
        manifest.format == "marreq.deployment-provenance.v1",
        "unsupported provenance format"
    );
    ensure!(
        manifest.variant == variant,
        "expected {variant} provenance variant"
    );
    let services = match variant {
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
    };
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
        let repository = if component == Frontend {
            "marreq-frontend"
        } else if component == Mcp {
            "marreq-mcp-server"
        } else {
            component.directory()
        };
        ensure!(
            artifact.image == format!("ghcr.io/marreqsw/{repository}"),
            "{component}: invalid image repository"
        );
        ensure!(
            artifact
                .digest
                .strip_prefix("sha256:")
                .is_some_and(|digest| nonzero_hex(digest, 64)),
            "{component}: invalid registry digest"
        );
        ensure!(
            artifact
                .source_tag
                .as_ref()
                .is_none_or(|tag| *tag == source_tag(component, &artifact.version)),
            "{component}: invalid source tag"
        );
        validate_contract(root, artifact, component)?;
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
