use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    ops::Range,
};

use anyhow::{Context, Result, ensure};
use serde_json::value::RawValue;
use toml_edit::{ImDocument, Item, Table};

use crate::{
    Repository, json,
    registry::{BACKENDS, Component, Registry, StableVersion},
};

pub const METADATA: [&str; 7] = [
    "Cargo.lock",
    "frontend/package-lock.json",
    "frontend/package.json",
    "marreq-cloud/Cargo.toml",
    "marreq-server/Cargo.toml",
    "mcp-server/package-lock.json",
    "mcp-server/package.json",
];

struct Edit {
    range: Range<usize>,
    replacement: String,
}

fn apply(text: &str, mut edits: Vec<Edit>) -> String {
    edits.sort_unstable_by_key(|edit| std::cmp::Reverse(edit.range.start));
    let mut result = text.to_owned();
    for edit in edits {
        result.replace_range(edit.range, &edit.replacement);
    }
    result
}

type Members<'a> = BTreeMap<String, &'a RawValue>;

fn npm(text: &str, component: Component, expected: &StableVersion, lock: bool) -> Result<String> {
    let data = json::value(text.as_bytes())?;
    ensure!(data.is_object(), "expected npm package object");
    let root: Members<'_> = serde_json::from_str(text)?;
    let mut packages = vec![(&data, &root)];
    let local;
    if lock {
        ensure!(
            data["lockfileVersion"].as_u64() == Some(3) && data["packages"].is_object(),
            "expected npm lockfile v3"
        );
        let members: Members<'_> = serde_json::from_str(root["packages"].get())?;
        let package = members
            .get("")
            .context("missing root npm lockfile package")?;
        local = serde_json::from_str(package.get())?;
        packages.push((&data["packages"][""], &local));
    }

    let mut edits = Vec::new();
    let name = format!("marreq-{}", component.directory());
    for (package, members) in packages {
        ensure!(
            package["name"].as_str() == Some(&name),
            "expected metadata for {name}"
        );
        let old: StableVersion = package["version"]
            .as_str()
            .context("expected stable SemVer X.Y.Z")?
            .parse()?;
        if old != *expected {
            // RawValue borrows the exact JSON token from the original text.
            let raw = members["version"].get();
            let start = raw.as_ptr() as usize - text.as_ptr() as usize;
            edits.push(Edit {
                range: start..start + raw.len(),
                replacement: format!("\"{expected}\""),
            });
        }
    }
    Ok(apply(text, edits))
}

fn cargo_version(
    text: &str,
    package: &Table,
    expected: &StableVersion,
    edits: &mut Vec<Edit>,
) -> Result<()> {
    let field = package.get("version").context("missing package version")?;
    let old = field.as_str().context("expected stable SemVer X.Y.Z")?;
    if old.parse::<StableVersion>()? != *expected {
        let range = field
            .span()
            .context("expected a version value with a source span")?;
        let key = package
            .key("version")
            .and_then(toml_edit::Key::span)
            .context("expected a version key with a source span")?;
        let literal = &text[range.clone()];
        ensure!(
            &text[key] == "version"
                && (literal == format!("\"{old}\"") || literal == format!("'{old}'")),
            "expected one plain version assignment"
        );
        edits.push(Edit {
            range: range.start + 1..range.end - 1,
            replacement: expected.to_string(),
        });
    }
    Ok(())
}

fn table_header<'a>(text: &'a str, table: &Table) -> Option<&'a str> {
    let start = table.span()?.start;
    if start != 0 && text.as_bytes()[start - 1] != b'\n' {
        return None;
    }
    text[start..].lines().next()
}

fn cargo(text: &str, component: Option<Component>, versions: &Registry) -> Result<String> {
    let data = ImDocument::parse(text)?;
    let mut edits = Vec::new();
    if let Some(component) = component {
        let package = data
            .get("package")
            .and_then(Item::as_table)
            .context("expected [package] metadata")?;
        ensure!(
            table_header(text, package).is_some_and(|line| line.trim_end() == "[package]"),
            "expected a plain [package] header"
        );
        ensure!(
            package.get("name").and_then(Item::as_str) == Some(component.directory()),
            "expected [package] metadata for {}",
            component.directory()
        );
        cargo_version(text, package, versions.component(component), &mut edits)?;
    } else {
        let entries = data
            .get("package")
            .and_then(Item::as_array_of_tables)
            .context("expected standard [[package]] lockfile entries")?;
        ensure!(
            entries
                .iter()
                .all(|entry| table_header(text, entry) == Some("[[package]]")),
            "expected standard [[package]] lockfile entries"
        );
        for component in BACKENDS {
            let name = component.directory();
            let mut matching = entries
                .iter()
                .filter(|package| package.get("name").and_then(Item::as_str) == Some(name));
            let entry = matching
                .next()
                .filter(|package| !package.contains_key("source"));
            let entry = entry
                .with_context(|| format!("expected exactly one local {name} lockfile entry"))?;
            ensure!(
                matching.next().is_none(),
                "expected exactly one local {name} lockfile entry"
            );
            cargo_version(text, entry, versions.component(component), &mut edits)?;
        }
        for package in entries {
            if let Some(dependencies) = package.get("dependencies") {
                let dependencies = dependencies
                    .as_array()
                    .context("expected a dependency list")?;
                for dependency in dependencies {
                    let dependency = dependency
                        .as_str()
                        .context("expected a string lockfile dependency")?;
                    ensure!(
                        !BACKENDS.iter().any(|component| dependency
                            .strip_prefix(component.directory())
                            .is_some_and(|suffix| suffix.starts_with(' '))),
                        "unsupported version-qualified workspace lockfile reference {dependency:?}"
                    );
                }
            }
        }
    }
    Ok(apply(text, edits))
}

pub(crate) fn update(path: &str, text: &str, versions: &Registry) -> Result<String> {
    let result = match path {
        "Cargo.lock" => cargo(text, None, versions),
        "marreq-cloud/Cargo.toml" => cargo(text, Some(Component::Cloud), versions),
        "marreq-server/Cargo.toml" => cargo(text, Some(Component::Server), versions),
        _ => {
            let component = if path.starts_with("frontend/") {
                Component::Frontend
            } else {
                Component::Mcp
            };
            npm(
                text,
                component,
                versions.component(component),
                path.ends_with("package-lock.json"),
            )
        }
    };
    result.with_context(|| path.to_owned())
}

struct Change {
    path: &'static str,
    original: String,
    updated: String,
}

fn changes(repo: &Repository, commit: Option<&str>, versions: &Registry) -> Result<Vec<Change>> {
    let mut changes = Vec::new();
    for path in METADATA {
        let original = repo.read(commit, path)?;
        let updated = update(path, &original, versions)?;
        if original != updated {
            changes.push(Change {
                path,
                original,
                updated,
            });
        }
    }
    Ok(changes)
}

pub(crate) fn mismatches(
    repo: &Repository,
    commit: Option<&str>,
    versions: &Registry,
) -> Result<Vec<&'static str>> {
    Ok(changes(repo, commit, versions)?
        .into_iter()
        .map(|change| change.path)
        .collect())
}

pub fn synchronize(repo: &Repository, check: bool) -> Result<(Registry, Vec<&'static str>)> {
    // Lock the authoritative file for cooperating writers. The check path
    // opens no files for writing and creates no temporary files.
    let lock;
    if !check {
        lock = OpenOptions::new()
            .read(true)
            .write(cfg!(windows))
            .open(repo.root().join("versions.toml"))?;
        lock.lock()?;
    }
    let registry_text = repo.worktree("versions.toml")?;
    let versions = Registry::parse(&registry_text)?;
    let changes = changes(repo, None, &versions)?;
    let paths: Vec<_> = changes.iter().map(|change| change.path).collect();
    if check {
        ensure!(
            paths.is_empty(),
            "out of sync: {}; run scripts/validate-release-versions sync",
            paths.join(", ")
        );
    } else {
        write_changes(repo, &registry_text, changes)?;
    }
    Ok((versions, paths))
}

fn write_changes(repo: &Repository, registry_text: &str, changes: Vec<Change>) -> Result<()> {
    let mut staged = Vec::new();
    for change in &changes {
        let target = repo.root().join(change.path);
        let parent = target
            .parent()
            .context("expected metadata parent directory")?;
        let mut pending = tempfile::NamedTempFile::new_in(parent)?;
        pending.write_all(change.updated.as_bytes())?;
        pending
            .as_file()
            .set_permissions(fs::metadata(&target)?.permissions())?;
        staged.push((pending, target));
    }
    ensure!(
        repo.worktree("versions.toml")? == registry_text,
        "versions.toml changed during synchronization"
    );
    for change in &changes {
        ensure!(
            repo.worktree(change.path)? == change.original,
            "{} changed during synchronization",
            change.path
        );
    }
    for (pending, target) in staged {
        pending
            .persist(&target)
            .with_context(|| target.display().to_string())?;
    }
    Ok(())
}
