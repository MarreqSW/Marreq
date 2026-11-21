use anyhow::{Result, ensure};
use toml::Value;

use crate::{Repository, registry::Registry, sync};

const MEMBER: &str = "tools/marreq-release";
const MEMBER_NAME: &str = "marreq-release";
const DOCKER_ANCHOR: &str = "COPY tools/marreq-migrations/Cargo.toml ./tools/marreq-migrations/\n";
const DOCKER_WIRING: &str = r#"COPY tools/marreq-release/Cargo.toml ./tools/marreq-release/
RUN mkdir -p tools/marreq-release/src && \
    for target in lib.rs main.rs; do \
        echo 'compile_error!("workspace placeholder must not be compiled");' > "tools/marreq-release/src/$target"; \
    done
"#;

pub(super) fn retired_release_input(path: &str) -> bool {
    // Recognize deletions from the old registry branch while rejecting this
    // layout in a candidate tree. The original commits remain valid baselines.
    path == "CHANGELOG"
        || matches!(
            path,
            "scripts/release_versions.py" | "scripts/test-release-versions.py"
        )
        || crate::changelog::retired_path(path)
}

pub(super) fn artifact_neutral(
    repo: &Repository,
    base: &str,
    head: &str,
    path: &str,
) -> Result<bool> {
    if !sync::METADATA.contains(&path) && !matches!(path, "Cargo.toml" | "docker/Dockerfile") {
        return Ok(false);
    }
    let before_mode = repo.mode(base, path)?;
    if before_mode.is_none() || before_mode != repo.mode(head, path)? {
        return Ok(false);
    }
    let before = repo.committed(base, path)?;
    let after = repo.committed(head, path)?;
    // Unsupported historical manifest shapes and substantive dependency
    // changes retain their conservative file-level artifact impact.
    Ok(match path {
        "Cargo.toml" => equal_projection(&before, &after, without_release_member),
        "docker/Dockerfile" => docker_projection(&after).as_deref() == Some(&before),
        _ => {
            let registry = Registry::neutral();
            let left = sync::update(path, &before, &registry);
            let right = sync::update(path, &after, &registry);
            match (left, right) {
                (Ok(left), Ok(right)) if path == "Cargo.lock" && left != right => {
                    equal_projection(&left, &right, without_release_lock_entry)
                }
                (Ok(left), Ok(right)) => left == right,
                _ => false,
            }
        }
    })
}

fn equal_projection(before: &str, after: &str, project: fn(&str) -> Result<(Value, bool)>) -> bool {
    match (project(before), project(after)) {
        (Ok((left, false)), Ok((right, true))) => left == right,
        _ => false,
    }
}

fn without_release_member(text: &str) -> Result<(Value, bool)> {
    let mut doc: Value = toml::from_str(text)?;
    let members = doc
        .get_mut("workspace")
        .and_then(|workspace| workspace.get_mut("members"))
        .and_then(Value::as_array_mut)
        .ok_or_else(|| anyhow::anyhow!("expected workspace members"))?;
    let count = members.len();
    members.retain(|member| member.as_str() != Some(MEMBER));
    ensure!(
        count - members.len() <= 1,
        "repeated release workspace member"
    );
    let removed = count != members.len();
    Ok((doc, removed))
}

fn without_release_lock_entry(text: &str) -> Result<(Value, bool)> {
    let mut doc: Value = toml::from_str(text)?;
    let packages = doc
        .get_mut("package")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| anyhow::anyhow!("expected Cargo packages"))?;
    let is_release = |package: &Value| {
        package.get("name").and_then(Value::as_str) == Some(MEMBER_NAME)
            && package.get("source").is_none()
    };
    let size = packages.len();
    packages.retain(|package| !is_release(package));
    let removed = size - packages.len();
    ensure!(removed <= 1, "repeated local release workspace lock entry");
    // If another package starts depending on the release tool its own lock
    // entry will change, so this projection will still report backend impact.
    Ok((doc, removed == 1))
}

fn docker_projection(text: &str) -> Option<String> {
    let (prefix, rest) = text.split_once(DOCKER_ANCHOR)?;
    let rest = rest.strip_prefix(DOCKER_WIRING)?;
    Some(format!("{prefix}{DOCKER_ANCHOR}{rest}"))
}
