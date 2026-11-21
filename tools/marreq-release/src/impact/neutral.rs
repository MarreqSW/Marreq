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
        "Cargo.toml" => matches!(
            (without_release_member(&before), without_release_member(&after)),
            (Ok((left, false)), Ok((right, true))) if left == right
        ),
        "docker/Dockerfile" => docker_projection(&after).as_deref() == Some(&before),
        _ => {
            let registry = Registry::neutral();
            let left = sync::update(path, &before, &registry);
            let right = sync::update(path, &after, &registry);
            match (left, right) {
                (Ok(left), Ok(right)) if path == "Cargo.lock" && left != right => {
                    matches!(
                        (without_release_lock_entry(&left), without_release_lock_entry(&right)),
                        (Ok((left, old_tool)), Ok((right, new_tool))) if left == right && old_tool != new_tool
                    )
                }
                (Ok(left), Ok(right)) => left == right,
                _ => false,
            }
        }
    })
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

fn without_release_lock_entry(text: &str) -> Result<(Value, Option<Value>)> {
    let mut doc: Value = toml::from_str(text)?;
    let packages = doc
        .get_mut("package")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| anyhow::anyhow!("expected Cargo packages"))?;
    let is_release = |package: &Value| {
        package.get("name").and_then(Value::as_str) == Some(MEMBER_NAME)
            && package.get("source").is_none()
    };
    let release = packages
        .iter()
        .position(is_release)
        .map(|index| packages.remove(index));
    ensure!(
        !packages.iter().any(is_release),
        "repeated local release workspace lock entry"
    );
    // Release-only changes are neutral only while the release tool is
    // independent of the rest of the workspace's dependency graphs.
    for dependency in packages
        .iter()
        .filter_map(|package| package.get("dependencies").and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_str)
    {
        ensure!(
            dependency != MEMBER_NAME
                && !dependency
                    .strip_prefix(MEMBER_NAME)
                    .is_some_and(|suffix| suffix.starts_with(' ')),
            "workspace package depends on the release tool"
        );
    }
    Ok((doc, release))
}

fn docker_projection(text: &str) -> Option<String> {
    let (prefix, rest) = text.split_once(DOCKER_ANCHOR)?;
    let rest = rest.strip_prefix(DOCKER_WIRING)?;
    Some(format!("{prefix}{DOCKER_ANCHOR}{rest}"))
}
