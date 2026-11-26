use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{Repository, json};

mod neutral;

const MAP: &str = "scripts/release-impact-map.json";
const ARTIFACTS: &[&str] = &["server", "cloud", "frontend", "mcp"];
const CHECKS: &[&str] = &[
    "api_contracts",
    "database_migrations",
    "deployment",
    "frontend",
    "mcp",
    "release_pipeline",
    "rust",
];
const INFRASTRUCTURE: &[&str] = &["build_and_ci", "deployment", "release"];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    schema_version: u32,
    rules: Vec<Rule>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    paths: Vec<String>,
    #[serde(default, deserialize_with = "nonempty")]
    artifacts: Vec<String>,
    #[serde(default, deserialize_with = "nonempty")]
    checks: Vec<String>,
    #[serde(default, deserialize_with = "nonempty")]
    infrastructure: Vec<String>,
}

fn nonempty<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    let values = Vec::deserialize(deserializer)?;
    if values.is_empty() {
        return Err(serde::de::Error::custom("expected nonempty rule dimension"));
    }
    Ok(values)
}

type Rules = Vec<(Vec<Regex>, Rule)>;

fn rules(repo: &Repository, commit: &str) -> Result<Rules> {
    let config: Config = json::decode(repo.committed(commit, MAP)?.as_bytes())?;
    ensure!(
        config.schema_version == 1 && !config.rules.is_empty(),
        "expected map schema version 1 and a nonempty rule list"
    );
    let mut seen = BTreeSet::new();
    let mut rules = Vec::new();
    for (index, rule) in config.rules.into_iter().enumerate() {
        let index = index + 1;
        ensure!(!rule.paths.is_empty(), "rule {index} needs path patterns");
        let mut patterns = Vec::new();
        for pattern in &rule.paths {
            ensure!(
                !pattern.is_empty() && seen.insert(pattern.clone()),
                "rule {index} has an empty, invalid or duplicate pattern"
            );
            patterns.push(Regex::new(&format!(r"\A(?:{pattern})\z"))?);
        }
        for (dimension, values, allowed) in [
            ("artifacts", &rule.artifacts, ARTIFACTS),
            ("checks", &rule.checks, CHECKS),
            ("infrastructure", &rule.infrastructure, INFRASTRUCTURE),
        ] {
            let unique: BTreeSet<_> = values.iter().collect();
            ensure!(
                unique.len() == values.len()
                    && values.iter().all(|value| allowed.contains(&value.as_str())),
                "rule {index} has invalid {dimension}"
            );
        }
        rules.push((patterns, rule));
    }
    Ok(rules)
}

fn classify<'a>(rules: &'a Rules, path: &str) -> Option<&'a Rule> {
    rules
        .iter()
        .find(|(patterns, _)| patterns.iter().any(|pattern| pattern.is_match(path)))
        .map(|(_, rule)| rule)
}

type Paths = BTreeMap<String, Vec<String>>;

#[derive(Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub base_sha: String,
    pub head_sha: String,
    pub artifacts: Paths,
    pub checks: Paths,
    pub infrastructure: Paths,
    pub unknown_inputs: BTreeSet<String>,
}

fn tree(repo: &Repository, commit: &str) -> Result<BTreeMap<Vec<u8>, bool>> {
    let bytes = repo.git(&["ls-tree", "-rz", "--full-tree", commit])?;
    bytes
        .split(|byte| *byte == 0)
        .filter(|row| !row.is_empty())
        .map(|row| {
            let tab = row
                .iter()
                .position(|byte| *byte == b'\t')
                .context("invalid Git tree entry")?;
            let (metadata, path) = row.split_at(tab);
            let path = &path[1..];
            let mode = metadata.split(|byte| *byte == b' ').next();
            Ok((path.to_vec(), matches!(mode, Some(b"100644" | b"100755"))))
        })
        .collect()
}

fn add(dimension: &mut Paths, values: &[String], path: &str) {
    for value in values {
        dimension
            .entry(value.clone())
            .or_default()
            .push(path.to_owned());
    }
}

fn report_path(path: &[u8]) -> String {
    match std::str::from_utf8(path) {
        Ok(path) => path.to_owned(),
        Err(_) => path.escape_ascii().to_string(),
    }
}

pub fn detect(repo: &Repository, base: &str, head: &str) -> Result<Report> {
    let base = repo.resolve(base)?;
    let head = repo.resolve(head)?;
    let patterns = rules(repo, &head).with_context(|| format!("invalid {MAP} at {head}"))?;
    let head_tree = tree(repo, &head)?;
    let base_tree = tree(repo, &base)?;
    // A rename affects both boundaries, regardless of Git's similarity settings.
    let diff = repo.git(&[
        "diff",
        "--name-only",
        "-z",
        "--no-renames",
        "--no-relative",
        "--no-ext-diff",
        "--ignore-submodules=none",
        &base,
        &head,
        "--",
    ])?;
    let changed: BTreeSet<_> = diff
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .collect();
    let mut unknown: BTreeSet<&[u8]> = head_tree
        .iter()
        .filter(|(path, regular)| {
            !**regular
                || std::str::from_utf8(path)
                    .ok()
                    .and_then(|path| classify(&patterns, path))
                    .is_none()
        })
        .map(|(path, _)| path.as_slice())
        .collect();
    unknown.extend(
        base_tree
            .iter()
            .filter(|(path, regular)| !**regular && changed.contains(path.as_slice()))
            .map(|(path, _)| path.as_slice()),
    );
    let mut report = Report {
        schema_version: 1,
        base_sha: base,
        head_sha: head,
        artifacts: Paths::new(),
        checks: Paths::new(),
        infrastructure: Paths::new(),
        unknown_inputs: unknown.iter().map(|path| report_path(path)).collect(),
    };
    for path in changed {
        if unknown.contains(path) {
            continue;
        }
        let Ok(path) = std::str::from_utf8(path) else {
            report.unknown_inputs.insert(report_path(path));
            continue;
        };
        if let Some(rule) = classify(&patterns, path) {
            if !rule.artifacts.is_empty()
                && !neutral::artifact_neutral(repo, &report.base_sha, &report.head_sha, path)?
            {
                add(&mut report.artifacts, &rule.artifacts, path);
            }
            add(&mut report.checks, &rule.checks, path);
            add(&mut report.infrastructure, &rule.infrastructure, path);
        } else if !head_tree.contains_key(path.as_bytes()) && neutral::retired_release_input(path) {
            add(&mut report.checks, &["release_pipeline".into()], path);
            add(&mut report.infrastructure, &["release".into()], path);
        } else {
            report.unknown_inputs.insert(path.to_owned());
        }
    }
    Ok(report)
}
