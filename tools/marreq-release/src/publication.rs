use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{
    json as strict_json,
    provenance::{self, Artifact, ContractReference, Manifest},
    registry::{BACKENDS, COMPONENTS, Component, Registry, StableVersion},
    validation::{Action, Decisions},
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Selected {
    version: StableVersion,
    action: Action,
    source_sha: String,
    source_tag: String,
    image: String,
    contract_file: String,
    retained_digest: Option<String>,
    retained_contract_sha256: Option<String>,
}

fn contract_file(component: Component) -> String {
    format!("{component}-contract.json")
}

pub fn selection(head: &str, registry: &Registry, decisions: &Decisions) -> Value {
    let entries: BTreeMap<_, _> = COMPONENTS
        .into_iter()
        .map(|component| {
            let decision = &decisions[&component];
            let version = registry.component(component);
            let reuse = decision
                .baseline
                .as_ref()
                .filter(|_| decision.action == Action::Reuse);
            (
                component,
                Selected {
                    version: version.clone(),
                    action: decision.action,
                    source_sha: reuse.map_or(head, |record| &record.source_sha).to_owned(),
                    source_tag: provenance::source_tag(component, version),
                    image: provenance::image(component),
                    contract_file: contract_file(component),
                    retained_digest: reuse.map(|record| record.digest.clone()),
                    retained_contract_sha256: reuse.map(|record| record.contract_sha256.clone()),
                },
            )
        })
        .collect();
    json!(entries)
}

#[derive(Deserialize)]
struct Plan {
    tag: String,
    head: String,
    previous: String,
    bootstrap: bool,
    versions: Versions,
    selection: BTreeMap<Component, Selected>,
}

#[derive(Deserialize)]
struct Versions {
    marreq: StableVersion,
    #[serde(flatten)]
    components: BTreeMap<Component, StableVersion>,
}

pub struct Run {
    pub digests: PathBuf,
    pub id: u64,
    pub timestamp: i64,
    pub tag_object: String,
}

fn write(path: &Path, content: &impl Serialize) -> Result<String> {
    let mut bytes = serde_json::to_vec_pretty(content)?;
    bytes.push(b'\n');
    fs::write(path, &bytes)?;
    Ok(hex::encode(Sha256::digest(&bytes)))
}

fn contract(
    directory: &Path,
    component: Component,
    selected: &Selected,
) -> Result<ContractReference> {
    let file = contract_file(component);
    ensure!(
        selected.contract_file == file,
        "{component}: invalid contract filename"
    );
    let sha256 = hex::encode(Sha256::digest(fs::read(directory.join(&file))?));
    if selected.action == Action::Reuse {
        ensure!(
            selected.retained_contract_sha256.as_ref() == Some(&sha256),
            "{component}: retained contract checksum mismatch"
        );
    }
    let reference = ContractReference { file, sha256 };
    provenance::validate_contract(
        directory,
        &reference,
        component,
        &selected.version,
        &selected.source_sha,
    )
    .with_context(|| format!("{component}: invalid selected contract"))?;
    Ok(reference)
}

pub fn write_assets(plan: &Path, directory: &Path, run: Option<Run>) -> Result<()> {
    let plan: Plan = strict_json::decode(&fs::read(plan)?)?;
    let directory = directory.canonicalize()?;
    ensure!(
        plan.tag == format!("v{}", plan.versions.marreq)
            && provenance::full_sha(&plan.head)
            && provenance::full_sha(&plan.previous)
            && plan.selection.len() == COMPONENTS.len()
            && plan.versions.components.len() == COMPONENTS.len(),
        "invalid product plan identity"
    );
    let mut contracts = BTreeMap::new();
    for component in COMPONENTS {
        let entry = plan
            .selection
            .get(&component)
            .context("missing component selection")?;
        ensure!(
            plan.versions.components.get(&component) == Some(&entry.version)
                && entry.source_tag == provenance::source_tag(component, &entry.version)
                && entry.image == provenance::image(component)
                && provenance::full_sha(&entry.source_sha),
            "{component}: invalid component identity"
        );
        match entry.action {
            Action::Build => ensure!(
                entry.source_sha == plan.head
                    && entry.retained_digest.is_none()
                    && entry.retained_contract_sha256.is_none(),
                "{component}: new build must use the candidate source"
            ),
            Action::Reuse => ensure!(
                !plan.bootstrap
                    && entry
                        .retained_digest
                        .as_deref()
                        .is_some_and(provenance::digest),
                "{component}: reuse requires a trusted published digest"
            ),
        }
        contracts.insert(component, contract(&directory, component, entry)?);
    }

    let descriptor = |component| {
        let entry = &plan.selection[&component];
        let reference = &contracts[&component];
        json!({"file": reference.file, "sha256": reference.sha256, "version": entry.version, "source_sha": entry.source_sha})
    };
    let mut compatibility = BTreeMap::new();
    for variant in BACKENDS {
        let file = format!("{variant}-compatibility.json");
        let report = json!({
            "format": "marreq.deployment-contract.v2",
            "variant": variant,
            "backend": descriptor(variant),
            "frontend": descriptor(Component::Frontend),
            "mcp": descriptor(Component::Mcp),
        });
        let sha256 = write(&directory.join(&file), &report)?;
        compatibility.insert(variant, json!({"file": file, "sha256": sha256}));
    }
    let Some(run) = run else {
        return Ok(());
    };
    ensure!(
        run.id > 0
            && provenance::full_sha(&run.tag_object)
            && run.timestamp > 0
            && time::OffsetDateTime::from_unix_timestamp(run.timestamp).is_ok(),
        "invalid release run identity or UTC timestamp"
    );
    let digests: BTreeMap<Component, String> = strict_json::decode(&fs::read(run.digests)?)?;
    ensure!(
        digests.len() == COMPONENTS.len(),
        "every component needs a digest"
    );
    let mut components = BTreeMap::new();
    for component in COMPONENTS {
        let digest = digests
            .get(&component)
            .context("missing component digest")?;
        let entry = &plan.selection[&component];
        ensure!(
            provenance::digest(digest)
                && entry
                    .retained_digest
                    .as_ref()
                    .is_none_or(|retained| retained == digest),
            "{component}: invalid digest or substitution of a retained artifact"
        );
        components.insert(
            component,
            json!({
                "version": entry.version,
                "source_sha": entry.source_sha,
                "source_tag": entry.source_tag,
                "image": entry.image,
                "digest": digest,
                "reference": format!("{}@{digest}", entry.image),
                "action": entry.action,
                "contract": contracts[&component]
            }),
        );
    }

    let mut provenance = BTreeMap::new();
    for variant in BACKENDS {
        let file = format!("{variant}-provenance.json");
        let artifacts = provenance::services(variant)
            .into_iter()
            .map(|(component, service)| {
                let entry = &plan.selection[&component];
                (
                    component,
                    Artifact {
                        service: service.to_owned(),
                        version: entry.version.clone(),
                        source_sha: entry.source_sha.clone(),
                        source_tag: Some(entry.source_tag.clone()),
                        image: entry.image.clone(),
                        digest: digests[&component].clone(),
                        contract: contracts[&component].clone(),
                    },
                )
            })
            .collect();
        let sha256 = write(
            &directory.join(&file),
            &Manifest {
                format: "marreq.deployment-provenance.v1".into(),
                variant,
                product_release: Some(plan.versions.marreq.clone()),
                components: artifacts,
            },
        )?;
        provenance.insert(variant, json!({"file": file, "sha256": sha256}));
    }
    write(
        &directory.join("release-manifest.json"),
        &json!({
            "schema_version": 1,
            "product": {"version": plan.versions.marreq, "tag": plan.tag, "source_sha": plan.head, "tag_object": run.tag_object},
            "recorded_at_unix": run.timestamp,
            "workflow": {
                "repository": "MarreqSW/Marreq",
                "path": ".github/workflows/release-artifacts.yml",
                "run_id": run.id
            },
            "validation": {
                "previous_source_sha": plan.previous,
                "bootstrap": plan.bootstrap,
                "compatibility": compatibility,
                "deployment_provenance": provenance
            },
            "components": components
        }),
    )?;
    Ok(())
}
