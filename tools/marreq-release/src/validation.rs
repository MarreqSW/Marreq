use std::{collections::BTreeMap, path::Path, sync::LazyLock};

use anyhow::{Result, ensure};
use regex::Regex;
use serde::Serialize;

use crate::{
    Repository,
    changelog::{self, Histories},
    impact,
    provenance::{self, Baseline, Records},
    registry::{COMPONENTS, Component, Registry, StableVersion},
    sync,
};

static VERSION_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\A(?:0|[1-9][0-9]{0,15})(?:\.(?:0|[1-9][0-9]{0,15})){2}\z")
        .expect("version tag regex")
});

// This tag predates coordinated releases. Keep the exception tied to its
// audited commit so moving the tag cannot silently bypass release validation.
const LEGACY_PRODUCT_TAG: &str = "v1.0.0";
const LEGACY_PRODUCT_TAG_SHA: &str = "14348c644b302229f680ce2f23093f715c029e6b";

fn is_historical_product_tag(tag: &str, sha: &str) -> bool {
    tag == LEGACY_PRODUCT_TAG && sha == LEGACY_PRODUCT_TAG_SHA
}

fn verify(repo: &Repository, head: Option<&str>) -> Result<(Registry, Histories)> {
    let registry = Registry::parse(&repo.read(head, "versions.toml")?)?;
    let histories = changelog::read(repo, head, &registry)?;
    verify_metadata(repo, head, &registry)?;
    Ok((registry, histories))
}

fn verify_metadata(repo: &Repository, head: Option<&str>, registry: &Registry) -> Result<()> {
    let changes = sync::mismatches(repo, head, registry)?;
    ensure!(
        changes.is_empty(),
        "version metadata differs in {}; run scripts/validate-release-versions sync",
        changes.join(", ")
    );
    Ok(())
}

pub fn check(
    repo: &Repository,
    head: Option<&str>,
    base: Option<&str>,
) -> Result<(Registry, bool)> {
    let head = head.map(|head| repo.resolve(head)).transpose()?;
    let (registry, histories) = verify(repo, head.as_deref())?;
    let mut bootstrap = false;
    if let Some(base) = base {
        let base = repo.resolve(base)?;
        let ancestry_head = match &head {
            Some(head) => head.clone(),
            None => repo.resolve("HEAD")?,
        };
        repo.ancestor(&base, &ancestry_head)?;
        if repo.mode(&base, "versions.toml")?.is_some() {
            let previous = Registry::parse(&repo.committed(&base, "versions.toml")?)?;
            let prior_histories = changelog::previous(repo, &base, &previous)?;
            changelog::progress(&prior_histories, &histories, &previous, &registry)?;
        } else {
            ensure!(
                histories[&None].is_initial(registry.product())
                    && COMPONENTS
                        .into_iter()
                        .all(|component| histories[&Some(component)]
                            .is_initial(registry.component(component))),
                "registry introduction requires one explicit Bootstrap entry per changelog"
            );
            changelog::preserve_legacy_archive(repo, &base, &histories)?;
            bootstrap = true;
        }
    }
    Ok((registry, bootstrap))
}

fn verify_tag(tag: &str, version: &StableVersion) -> Result<()> {
    ensure!(
        tag == format!("v{version}"),
        "product tag must be v{version}, got {tag:?}"
    );
    Ok(())
}

pub fn tag(repo: &Repository, tag: &str, head: Option<&str>) -> Result<Registry> {
    let head = head.map(|head| repo.resolve(head)).transpose()?;
    let (registry, _) = verify(repo, head.as_deref())?;
    verify_tag(tag, registry.product())?;
    Ok(registry)
}

fn existing_tags(
    repo: &Repository,
    prior: &StableVersion,
    current: &StableVersion,
    records: &Records,
    head: &str,
) -> Result<()> {
    let tags = repo.git(&["for-each-ref", "--format=%(refname:strip=2)", "refs/tags"])?;
    for tag in std::str::from_utf8(&tags)?.lines() {
        if tag == LEGACY_PRODUCT_TAG {
            let source_sha = repo.resolve(&format!("refs/tags/{tag}"))?;
            ensure!(
                is_historical_product_tag(tag, &source_sha),
                "historical product tag {tag} has moved from its audited source commit"
            );
            continue;
        }
        if let Some(version) = tag
            .strip_prefix('v')
            .filter(|version| VERSION_TAG.is_match(version))
            && version.parse::<StableVersion>()? > *prior
        {
            ensure!(
                tag == format!("v{current}") && repo.resolve(&format!("refs/tags/{tag}"))? == head,
                "product tag {tag} is already reserved or newer than the supplied baseline"
            );
        }
        for component in COMPONENTS {
            if let Some(version) = tag
                .strip_prefix(&format!("marreq-{component}-v"))
                .filter(|version| VERSION_TAG.is_match(version))
            {
                ensure!(
                    version.parse::<StableVersion>()? <= records[&component].version,
                    "{component}: {tag} is newer than the supplied publication baseline"
                );
            }
        }
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum Action {
    Build,
    Reuse,
}

#[derive(Serialize)]
pub struct Decision {
    action: Action,
    baseline: Baseline,
    changed_inputs: Vec<String>,
}

pub type Decisions = BTreeMap<Component, Decision>;

fn decisions(
    repo: &Repository,
    previous: &str,
    head: &str,
    coordinated: bool,
    current: &Registry,
    history: &Histories,
    records: Records,
) -> Result<Decisions> {
    let mut decisions = BTreeMap::new();
    let mut reports = BTreeMap::new();
    for (component, baseline) in records {
        let source = &baseline.source_sha;
        repo.ancestor(source, if coordinated { previous } else { head })?;
        let report = match reports.entry(source.clone()) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(impact::detect(repo, source, head)?)
            }
        };
        ensure!(
            report.unknown_inputs.is_empty(),
            "Unclassified or unsupported release inputs: {:?}",
            report.unknown_inputs
        );
        let changed_inputs = report
            .artifacts
            .get(component.name())
            .cloned()
            .unwrap_or_default();
        let version = current.component(component);
        let old = &baseline.version;
        ensure!(
            version >= old,
            "{component}: candidate regresses below published {old}"
        );
        let action = if changed_inputs.is_empty() {
            ensure!(
                version == old,
                "{component}: no changed artifact inputs since {source}; retain {old}"
            );
            Action::Reuse
        } else {
            ensure!(
                version > old,
                "{component}: advance {old} for changed inputs: {changed_inputs:?}"
            );
            ensure!(
                history[&Some(component)].dated(version),
                "{component}: new artifact requires a dated changelog entry"
            );
            Action::Build
        };
        decisions.insert(
            component,
            Decision {
                baseline,
                action,
                changed_inputs,
            },
        );
    }
    Ok(decisions)
}

pub fn release(
    repo: &Repository,
    head: &str,
    tag: &str,
    previous: &str,
    evidence: [Option<&Path>; 2],
) -> Result<(Registry, String, Decisions)> {
    let head = repo.resolve(head)?;
    let (current, history) = verify(repo, Some(&head))?;
    verify_tag(tag, current.product())?;
    ensure!(
        provenance::full_sha(previous),
        "--previous needs the full SHA of the trusted prior product or bootstrap commit"
    );
    let previous = repo.resolve(previous)?;
    repo.ancestor(&previous, &head)?;
    let prior = Registry::parse(&repo.committed(&previous, "versions.toml")?)?;
    let prior_history = changelog::previous(repo, &previous, &prior)?;
    verify_metadata(repo, Some(&previous), &prior)?;
    changelog::preserve(&prior_history, &history)?;
    ensure!(
        current.product() > prior.product(),
        "product version must advance beyond {}",
        prior.product()
    );
    ensure!(
        history[&None].dated(current.product()),
        "candidate needs a dated product release entry"
    );
    let prior_product = if prior_history[&None].is_bootstrap(prior.product()) {
        let parent = repo.resolve(&format!("{previous}^1"))?;
        ensure!(
            repo.mode(&parent, "versions.toml")?.is_none(),
            "--previous must be the commit introducing the bootstrap registry"
        );
        None
    } else {
        ensure!(
            repo.resolve(&format!("refs/tags/v{}", prior.product()))? == previous,
            "prior product tag does not match --previous"
        );
        Some(prior.product())
    };
    let records = provenance::publications(repo, evidence, &prior, prior_product)?;
    existing_tags(repo, prior.product(), current.product(), &records, &head)?;
    let decisions = decisions(
        repo,
        &previous,
        &head,
        prior_product.is_some(),
        &current,
        &history,
        records,
    )?;
    Ok((current, head, decisions))
}

#[cfg(test)]
mod tests {
    use super::{LEGACY_PRODUCT_TAG, LEGACY_PRODUCT_TAG_SHA, is_historical_product_tag};

    #[test]
    fn historical_product_tag_exception_is_pinned_to_its_source_commit() {
        assert!(is_historical_product_tag(LEGACY_PRODUCT_TAG, LEGACY_PRODUCT_TAG_SHA));
        assert!(!is_historical_product_tag("v1.0.1", LEGACY_PRODUCT_TAG_SHA));
        assert!(!is_historical_product_tag(LEGACY_PRODUCT_TAG, "0000000000000000000000000000000000000000"));
    }
}
