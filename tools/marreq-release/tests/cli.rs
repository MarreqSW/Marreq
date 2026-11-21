mod support;

use std::{collections::BTreeSet, fs};

use marreq_release::{
    changelog,
    registry::Component::{Cloud, Frontend, Mcp, Server},
    sync::METADATA,
};
use serde_json::{Value, json};
use support::{BASE, Fixture, NEXT};

#[test]
fn valid_registry_bootstrap_and_cli_contract() {
    let fixture = Fixture::new();
    let versions =
        json!({"marreq": BASE, "server": BASE, "cloud": BASE, "frontend": BASE, "mcp": BASE});
    assert_eq!(
        fixture.call(&["check"], None),
        json!({"status": "ok", "versions": versions})
    );
    assert_eq!(
        fixture.call(
            &[
                "check",
                "--head",
                &fixture.bootstrap,
                "--base",
                &fixture.legacy
            ],
            None
        ),
        json!({"status": "ok", "versions": versions, "bootstrap": true})
    );
    assert_eq!(
        fixture.call(&["sync", "--check"], None),
        json!({"status": "ok", "versions": versions, "updated_paths": []})
    );
    fixture.call(&["tag", "v0.1.0"], None);
    let invalid = fixture.call(&["tag", "v0.1.1"], Some("must be v0.1.0"));
    assert_eq!(invalid.as_object().unwrap().len(), 2);
    let text = fixture.command(&["tag", "wrong"]);
    assert_eq!(text.status.code(), Some(2));
    assert!(text.stdout.is_empty());
    assert!(String::from_utf8_lossy(&text.stderr).starts_with("Release versions: "));
    assert_eq!(fixture.command(&["release"]).status.code(), Some(2));
    assert_eq!(fixture.command(&["--help"]).status.code(), Some(0));
}

#[test]
fn invalid_registry_values_and_schema() {
    let fixture = Fixture::new();
    let original = fixture.read("versions.toml");
    for invalid in [
        original.replace("mcp = \"0.1.0\"\n", ""),
        format!("{original}core = \"0.1.0\"\n"),
        format!("{original}\n[unknown]\n"),
        format!("{original}mcp = \"0.1.0\"\n"),
        format!("{original}\n[marreq]\n"),
    ]
    .into_iter()
    .chain(
        [
            "1",
            "\"1.0\"",
            "\"01.0.0\"",
            "\"1.0.0-rc.1\"",
            "\"1.0.0+metadata\"",
            "\"0.1.0",
            "\"9999999999999999.0.0\"",
        ]
        .map(|bad| original.replace("\"0.1.0\"", bad)),
    ) {
        fixture.write("versions.toml", invalid);
        fixture.call(&["check"], Some("versions.toml"));
    }
}

#[test]
fn package_inconsistency_and_idempotent_sync() {
    let fixture = Fixture::new();
    for path in METADATA {
        let original = fixture.read(path);
        let divergent = original.replacen(BASE, NEXT, 1);
        fixture.write(path, &divergent);
        fixture.call(&["check"], Some(path));
        fixture.call(&["sync", "--check"], Some(path));
        assert_eq!(fixture.read(path), divergent);
        assert_eq!(
            fixture.call(&["sync"], None)["updated_paths"],
            json!([path])
        );
        assert_eq!(fixture.read(path), original);
    }
    let lock = "mcp-server/package-lock.json";
    let mut parsed: Value = serde_json::from_str(&fixture.read(lock)).unwrap();
    parsed["packages"][""]["version"] = json!(NEXT);
    fixture.write_json(lock, &parsed);
    fixture.call(&["check"], Some(lock));
    fixture.call(&["sync"], None);
    fixture.set_versions(&["server", "mcp"], NEXT);
    let before = METADATA.map(|path| fixture.read(path));
    assert_eq!(
        fixture.call(&["sync"], None)["updated_paths"],
        json!([
            "Cargo.lock",
            "marreq-server/Cargo.toml",
            "mcp-server/package-lock.json",
            "mcp-server/package.json"
        ])
    );
    let after = METADATA.map(|path| fixture.read(path));
    assert_eq!(fixture.call(&["sync"], None)["updated_paths"], json!([]));
    assert_eq!(METADATA.map(|path| fixture.read(path)), after);
    for (original, updated) in before.iter().zip(after) {
        assert_eq!(*original, updated.replace(NEXT, BASE));
    }
}

#[test]
fn invalid_sync_never_partially_writes() {
    let fixture = Fixture::new();
    fixture.set_versions(&["server"], NEXT);
    let path = "mcp-server/package.json";
    for (text, error) in [
        (
            r#"{"name":"marreq-mcp-server","version":"0.1.0","version":"0.2.0"}"#,
            "duplicate JSON key",
        ),
        (
            r#"{"name":"marreq-mcp-server","version":"0.1.0","p":{"k":1,"\u006b":2}}"#,
            "duplicate JSON key",
        ),
    ] {
        fixture.write(path, text);
        let before = METADATA.map(|path| fixture.read(path));
        fixture.call(&["sync"], Some(error));
        assert_eq!(METADATA.map(|path| fixture.read(path)), before);
    }
    fixture.git(&["restore", path]);
    let lock = fixture
        .read("Cargo.lock")
        .replace("\"example\"", "\"marreq-server 0.1.0\"");
    fixture.write("Cargo.lock", &lock);
    fixture.call(&["sync"], Some("unsupported version-qualified"));
    assert_eq!(fixture.read("Cargo.lock"), lock);
}

#[test]
fn formatting_and_unrelated_values_survive_sync() {
    let fixture = Fixture::new();
    let path = "frontend/package-lock.json";
    let original = fixture
        .read(path)
        .replace("\"version\": \"0.1.0\"", r#""v\u0065rsion" : "0.1.0" "#)
        .replace('\n', "\r\n");
    fixture.write(path, &original);
    fixture.set_versions(&["frontend"], NEXT);
    fixture.call(&["sync"], None);
    assert_eq!(
        fixture.read(path),
        original.replace("\"0.1.0\"", "\"0.1.1\"")
    );
}

#[test]
fn malformed_or_missing_changelog_entries() {
    let fixture = Fixture::new();
    let path = "CHANGELOG.md";
    let original = fixture.read(path);
    for (suffix, error) in [
        (
            "\n## [0.1.0] - Bootstrap\n\n- Duplicate.\n",
            "duplicate version",
        ),
        ("\n## [Unreleased]\n", "duplicate or misplaced"),
        ("\n## [End]\n", "expected an Unreleased heading"),
        (
            "\n## [0.0.9] - 2026-02-31\n\n- Invalid date.\n",
            "invalid release date",
        ),
        (
            "\n## [0.1.3] - 2026-11-21\n\n- Out of order.\n",
            "versions must descend",
        ),
        (
            "\n## [0.0.9] - 0000-01-01\n\n- Invalid year.\n",
            "invalid release date",
        ),
    ] {
        fixture.write(path, format!("{original}{suffix}"));
        fixture.call(&["check"], Some(error));
    }
    fixture.write(
        path,
        original.replace("- Initial registry baseline.", "### Empty"),
    );
    fixture.call(&["check"], Some("needs release notes"));
    fixture.write(path, &original);
    fixture.set_versions(&["frontend"], NEXT);
    fixture.call(&["sync"], None);
    fixture.call(&["check"], Some("frontend/CHANGELOG.md: latest entry"));
    fixture.add_entry(Some(Frontend), NEXT);
    fixture.set_versions(&["marreq", "frontend"], NEXT);
    fixture.call(&["check"], Some("CHANGELOG.md: latest entry"));
}

#[test]
fn regressing_product_and_component_versions() {
    let fixture = Fixture::new();
    for (name, component) in [("marreq", None), ("cloud", Some(Cloud))] {
        fixture.reset();
        fixture.set_versions(&[name], "0.0.9");
        let path = changelog::path(component);
        fixture.write(&path, fixture.read(&path).replace(BASE, "0.0.9"));
        fixture.call(&["sync"], None);
        fixture.call(
            &["check", "--base", &fixture.bootstrap],
            Some(&format!("{name}: version regressed")),
        );
    }
}

#[test]
fn historical_notes_and_release_dates_are_immutable() {
    let fixture = Fixture::new();
    let first = fixture.prepare(&[]);
    for text in [
        fixture
            .read("CHANGELOG.md")
            .replace("2026-11-21", "2026-11-22"),
        fixture
            .read("CHANGELOG.md")
            .replace("- Release fixture.", "- Rewritten published notes."),
        fixture.read("CHANGELOG.md").replace(
            "- Initial registry baseline.",
            "- Rewritten bootstrap notes.",
        ),
    ] {
        fixture.write("CHANGELOG.md", text);
        fixture.call(
            &["check", "--base", &first],
            Some("historical changelog entry"),
        );
        fixture.git(&["restore", "CHANGELOG.md"]);
    }
    let notes = fixture
        .read("frontend/CHANGELOG.md")
        .replace("- Initial registry baseline.", "- Edited artifact history.");
    fixture.write("frontend/CHANGELOG.md", notes);
    fixture.release(&fixture.commit(), Some("historical changelog entry"));
}

#[test]
fn duplicate_product_and_unjustified_component_bumps() {
    let fixture = Fixture::new();
    fixture.release_from(
        &fixture.bootstrap,
        &fixture.bootstrap,
        "v0.1.0",
        Some("product version must advance"),
    );
    fixture.release(&fixture.prepare(&[Server]), Some("retain 0.1.0"));
}

fn build_set(report: &Value) -> BTreeSet<&str> {
    report["components"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(_, row)| row["action"] == "build")
        .map(|(name, _)| name.as_str())
        .collect()
}

#[test]
fn independent_component_changes() {
    let fixture = Fixture::new();
    for (component, path) in [
        (Mcp, "mcp-server/src/example.ts"),
        (Frontend, "frontend/src/example.ts"),
        (Server, "marreq-server/src/example.rs"),
        (Cloud, "marreq-cloud/src/example.rs"),
    ] {
        fixture.reset();
        fixture.write(path, "release input change\n");
        fixture.release(
            &fixture.prepare(&[]),
            Some(&format!("{component}: advance 0.1.0")),
        );
        fixture.reset();
        fixture.write(path, "release input change\n");
        let report = fixture.release(&fixture.prepare(&[component]), None);
        assert_eq!(build_set(&report), BTreeSet::from([component.name()]));
        assert_eq!(
            report["components"][component.name()]["changed_inputs"],
            json!([path])
        );
    }
}

#[test]
fn shared_core_lock_and_build_inputs() {
    let fixture = Fixture::new();
    for path in [
        "marreq-core/src/example.rs",
        "Cargo.lock",
        "docker/Dockerfile",
    ] {
        fixture.reset();
        if path == "Cargo.lock" {
            fixture.write(path, fixture.read(path).replace("1.2.3", "1.2.4"));
        } else {
            fixture.write(path, "new build input\n");
        }
        fixture.release(&fixture.prepare(&[Server]), Some("cloud: advance"));
        fixture.set_versions(&["marreq", "server", "cloud"], NEXT);
        fixture.add_entry(Some(Cloud), NEXT);
        fixture.call(&["sync"], None);
        let report = fixture.release(&fixture.commit(), None);
        assert_eq!(build_set(&report), BTreeSet::from(["server", "cloud"]));
    }
}

#[test]
fn documentation_and_contract_only_release_reuses_artifacts() {
    let fixture = Fixture::new();
    fixture.write("docs/releases.md", "documentation only\n");
    fixture.write("docs/developer/openapi.yaml", "validation only\n");
    let head = fixture.prepare(&[]);
    let report = fixture.release(&head, None);
    assert!(build_set(&report).is_empty());
    assert_eq!(report["head"], head);
    assert_eq!(report["components"]["server"]["changed_inputs"], json!([]));
    let row = &report["components"]["mcp"]["baseline"];
    assert_eq!(row.as_object().unwrap().len(), 6);
    assert_eq!(row["source_sha"], fixture.legacy);
    assert_eq!(row["source_tag"], "marreq-mcp-v0.1.0");
    assert_eq!(row["version"], BASE);
    assert_eq!(row["image"], "ghcr.io/marreqsw/marreq-mcp-server");
    assert_eq!(row["contract_sha256"].as_str().unwrap().len(), 64);
    assert!(row["digest"].as_str().unwrap().starts_with("sha256:"));
    let impact = fixture.impact(&fixture.legacy, &head);
    assert_eq!(
        impact["checks"]["api_contracts"],
        json!(["docs/developer/openapi.yaml"])
    );
}

#[test]
fn unknown_and_unsafe_inputs_fail_closed() {
    let fixture = Fixture::new();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("example.ts", fixture.root.join("frontend/src/unsafe.ts"))
            .unwrap();
        fixture.release(&fixture.prepare(&[]), Some("frontend/src/unsafe.ts"));
        fixture.reset();
    }
    fixture.write("unknown-input", "must fail closed");
    let head = fixture.prepare(&[]);
    fixture.release(&head, Some("unknown-input"));
    let result = fixture.command(&[
        "impact",
        "--base",
        &fixture.bootstrap,
        "--head",
        &head,
        "--format",
        "json",
    ]);
    assert_eq!(result.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<Value>(&result.stdout).unwrap()["unknown_inputs"],
        json!(["unknown-input"])
    );
}

#[test]
fn component_baselines_are_independent() {
    let mut fixture = Fixture::new();
    fixture.write("frontend/src/example.ts", "published by newer frontend\n");
    fixture.write(
        "mcp-server/src/example.ts",
        "still unpublished by older MCP\n",
    );
    let frontend = fixture.commit();
    fixture.publish(Frontend, &frontend, BASE);
    fixture.write_evidence(None);
    let report = fixture.release(&fixture.prepare(&[Mcp]), None);
    assert_eq!(
        report["components"]["frontend"]["baseline"]["source_sha"],
        frontend
    );
    assert_eq!(report["components"]["frontend"]["action"], "reuse");
    assert_eq!(
        report["components"]["mcp"]["baseline"]["source_sha"],
        fixture.legacy
    );
    assert_eq!(report["components"]["mcp"]["action"], "build");
}

#[test]
fn missing_evidence_stale_tags_and_reserved_product_tag() {
    let fixture = Fixture::new();
    let head = fixture.prepare(&[]);
    fs::remove_file(fixture.evidence.join("cloud-provenance.json")).unwrap();
    fixture.call(
        &["check", "--head", &head, "--base", &fixture.bootstrap],
        None,
    );
    fixture.release(&head, Some("cloud publication provenance"));
    fixture.call(
        &[
            "release",
            "--head",
            &head,
            "--tag",
            "v0.1.1",
            "--previous",
            &fixture.bootstrap,
        ],
        Some("missing trusted server"),
    );
    fixture.release_from(
        &head,
        &fixture.bootstrap[..12],
        "v0.1.1",
        Some("needs the full SHA"),
    );
    fixture.write_evidence(None);
    fixture.git(&["tag", "marreq-mcp-v0.1.2", &fixture.bootstrap]);
    fixture.release(&head, Some("newer than the supplied publication baseline"));
    fixture.git(&["tag", "-d", "marreq-mcp-v0.1.2"]);
    fixture.git(&["tag", "v0.1.1", &fixture.bootstrap]);
    fixture.release(&head, Some("already reserved"));
}

#[test]
fn tampered_publications_contracts_and_source_tags() {
    let fixture = Fixture::new();
    let head = fixture.prepare(&[]);
    let path = fixture.evidence.join("cloud-provenance.json");
    let original = fixture.read(&path);
    let mut data: Value = serde_json::from_str(&original).unwrap();
    data["components"]["mcp"]["digest"] = json!(format!("sha256:{}", "b".repeat(64)));
    fixture.write_json(&path, &data);
    fixture.release(&head, Some("conflicting publication evidence"));
    fixture.write(
        &path,
        original.replace(
            "\"variant\": \"cloud\"",
            "\"variant\": \"cloud\", \"variant\": \"cloud\"",
        ),
    );
    fixture.release(&head, Some("duplicate JSON key"));
    fixture.write_evidence(None);
    let path = fixture.evidence.join("frontend.json");
    let good_contract = fixture.read(&path);
    fixture.write(&path, format!("{good_contract}\n"));
    fixture.release(&head, Some("contract checksum mismatch"));
    fixture.write(path, good_contract);
    fixture.git(&["tag", "-f", "marreq-frontend-v0.1.0", &fixture.bootstrap]);
    fixture.release(&head, Some("disagrees with the retained publication SHA"));
}

#[test]
fn non_ancestor_publication_source_is_rejected() {
    let mut fixture = Fixture::new();
    let head = fixture.prepare(&[]);
    fixture.git(&["checkout", "--detach", &fixture.bootstrap]);
    fixture.write("frontend/src/example.ts", "unrelated tagged branch\n");
    fixture.publish(Frontend, &fixture.commit(), BASE);
    fixture.write_evidence(None);
    fixture.release(&head, Some("must be an ancestor"));
}

#[test]
fn subsequent_coordinated_release_uses_exact_prior_product_tag() {
    let fixture = Fixture::new();
    let first = fixture.prepare(&[]);
    fixture.git(&[
        "tag",
        "-a",
        "v0.1.1",
        "-m",
        "First coordinated release",
        &first,
    ]);
    fixture.write_evidence(Some(NEXT));
    fixture.set_versions(&["marreq"], "0.1.2");
    fixture.add_entry(None, "0.1.2");
    let second = fixture.commit();
    fixture.release_from(&second, &first, "v0.1.2", None);
    fixture.git(&["tag", "-f", "v0.1.1", &fixture.bootstrap]);
    fixture.release_from(
        &second,
        &first,
        "v0.1.2",
        Some("prior product tag does not match"),
    );
}

#[test]
fn historic_changelog_layout_can_be_migrated_without_rewriting_the_bootstrap_commit() {
    let fixture = Fixture::new();
    fixture.git(&["reset", "--hard", &fixture.legacy]);
    let unreleased = "# Changelog\n\n## [Unreleased]\n\n- Existing pending changes.\n\n";
    let archive = "## [0.1.0] - 05/03/2026\n\n### Added\n\n- Historical notes.\n";
    fixture.write("CHANGELOG", format!("{unreleased}{archive}"));
    let pre_registry = fixture.commit();
    fixture.set_versions(&["marreq"], "1.0.0");
    let bootstrap = "## [1.0.0] - Bootstrap\n\n- Reserve the historical v1.0.0 tag.\n";
    fixture.write("changelogs/marreq.md", format!(
        "# Marreq changelog\n\n## [Unreleased]\n\n{bootstrap}  Earlier product notes and accumulated unreleased changes are in [CHANGELOG](../CHANGELOG).\n"
    ));
    let component_bootstrap =
        "## [Unreleased]\n\n## [0.1.0] - Bootstrap\n\n- Initial artifact floor.\n";
    for component in [Cloud, Frontend, Mcp, Server] {
        fixture.write(format!("changelogs/{component}.md"), component_bootstrap);
    }
    let original_bootstrap = fixture.commit();

    fixture.write(
        "CHANGELOG.md",
        format!("{unreleased}{bootstrap}\n{archive}"),
    );
    for component in [Cloud, Frontend, Mcp, Server] {
        let old_path = fixture.root.join(format!("changelogs/{component}.md"));
        fs::rename(
            old_path,
            fixture.root.join(changelog::path(Some(component))),
        )
        .unwrap();
    }
    fs::remove_file(fixture.root.join("changelogs/marreq.md")).unwrap();
    fs::remove_file(fixture.root.join("CHANGELOG")).unwrap();
    let migrated = fixture.commit();
    let initial = fixture.call(
        &["check", "--head", &migrated, "--base", &pre_registry],
        None,
    );
    assert_eq!(initial["bootstrap"], true);
    fixture.call(
        &["check", "--head", &migrated, "--base", &original_bootstrap],
        None,
    );
    let impact = fixture.impact(&original_bootstrap, &migrated);
    assert_eq!(impact["artifacts"], json!({}));
    assert_eq!(impact["unknown_inputs"], json!([]));

    fixture.set_versions(&["marreq"], "1.0.1");
    fixture.add_entry(None, "1.0.1");
    let head = fixture.commit();
    assert!(
        build_set(&fixture.release_from(&head, &original_bootstrap, "v1.0.1", None)).is_empty()
    );
    let edited = fixture
        .read("CHANGELOG.md")
        .replace("Historical notes.", "Edited notes.");
    fixture.write("CHANGELOG.md", edited);
    fixture.call(
        &["check", "--base", &migrated],
        Some("historical legacy changelog notes"),
    );
    fixture.release_from(
        &fixture.commit(),
        &original_bootstrap,
        "v1.0.1",
        Some("historical legacy changelog notes"),
    );
}

#[test]
fn release_tooling_changelogs_and_mirrored_versions_only_trigger_checks() {
    let fixture = Fixture::new();
    for path in [
        "tools/marreq-release/Cargo.toml",
        "tools/marreq-release/src/main.rs",
        "tools/marreq-release/tests/cli.rs",
    ] {
        fixture.write(path, "release infrastructure");
    }
    fixture.set_versions(&["server", "frontend", "mcp"], NEXT);
    fixture.call(&["sync"], None);
    for component in [None, Some(Cloud), Some(Frontend), Some(Mcp), Some(Server)] {
        let path = changelog::path(component);
        fixture.write(
            &path,
            fixture
                .read(&path)
                .replace("## [Unreleased]\n", "## [Unreleased]\n\n- New notes.\n"),
        );
    }
    let head = fixture.commit();
    let report = fixture.impact(&fixture.bootstrap, &head);
    assert_eq!(report["artifacts"], json!({}));
    assert_eq!(
        report["checks"]["release_pipeline"],
        json!([
            "CHANGELOG.md",
            "frontend/CHANGELOG.md",
            "marreq-cloud/CHANGELOG.md",
            "marreq-server/CHANGELOG.md",
            "mcp-server/CHANGELOG.md",
            "tools/marreq-release/Cargo.toml",
            "tools/marreq-release/src/main.rs",
            "tools/marreq-release/tests/cli.rs",
            "versions.toml"
        ])
    );
    assert!(
        report["checks"]["rust"]
            .as_array()
            .unwrap()
            .contains(&json!("Cargo.lock"))
    );
}

#[test]
fn isolated_workspace_registration_preserves_backend_reuse() {
    let fixture = Fixture::new();
    let initial_manifest = "[workspace]\nmembers = [\"marreq-server\", \"marreq-cloud\"]\n";
    let dockerfile = include_str!("../../../docker/Dockerfile");
    let (prefix, wiring) = dockerfile
        .split_once("COPY tools/marreq-release/Cargo.toml")
        .unwrap();
    let (_, rest) = wiring.split_once("\n\n").unwrap();
    fixture.write("docker/Dockerfile", format!("{prefix}\n{rest}"));
    fixture.write("Cargo.toml", initial_manifest);
    let base = fixture.commit();
    fixture.write(
        "Cargo.toml",
        initial_manifest.replace(
            "\"marreq-cloud\"]",
            "\"marreq-cloud\", \"tools/marreq-release\"]",
        ),
    );
    fixture.write("Cargo.lock", format!("{}\n[[package]]\nname = \"marreq-release\"\nversion = \"0.1.0\"\ndependencies = [\"example\"]\n", fixture.read("Cargo.lock")));
    fixture.write("docker/Dockerfile", dockerfile);
    let head = fixture.commit();
    let report = fixture.impact(&base, &head);
    assert_eq!(report["artifacts"], json!({}));
    assert_eq!(
        report["checks"]["rust"],
        json!(["Cargo.lock", "Cargo.toml"])
    );
    assert_eq!(
        report["checks"]["database_migrations"],
        json!(["Cargo.lock", "Cargo.toml", "docker/Dockerfile"])
    );

    fixture.write(
        "Cargo.lock",
        fixture.read("Cargo.lock").replace("1.2.3", "1.2.4"),
    );
    fixture.write(
        "Cargo.toml",
        format!(
            "{}\n[workspace.dependencies]\nanyhow = \"1\"\n",
            fixture.read("Cargo.toml")
        ),
    );
    fixture.write(
        "docker/Dockerfile",
        dockerfile.replace("FROM rust:1-trixie", "FROM rust:1-bookworm"),
    );
    let report = fixture.impact(&base, &fixture.commit());
    let changes = json!(["Cargo.lock", "Cargo.toml", "docker/Dockerfile"]);
    assert_eq!(
        report["artifacts"],
        json!({"server": changes, "cloud": changes})
    );
}

#[cfg(unix)]
#[test]
fn unsupported_path_still_emits_an_impact_report() {
    use std::os::unix::ffi::OsStrExt;
    let fixture = Fixture::new();
    let path = std::path::Path::new(std::ffi::OsStr::from_bytes(
        b"frontend/src/non-utf8-\xff.ts",
    ));
    fixture.write(path, "unsupported");
    let head = fixture.commit();
    let result = fixture.command(&[
        "impact",
        "--base",
        &fixture.bootstrap,
        "--head",
        &head,
        "--format",
        "json",
    ]);
    assert_eq!(result.status.code(), Some(2));
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(
        report["unknown_inputs"],
        json!([r"frontend/src/non-utf8-\xff.ts"])
    );
    assert_eq!(report["artifacts"], json!({}));
}
