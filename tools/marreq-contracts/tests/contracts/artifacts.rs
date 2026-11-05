use super::fixtures::{response, spec};
use marreq_contracts::{
    artifact::{self, Artifact, Bundle, FrontendRange, Plan, Policy},
    contract,
};
use serde_json::json;
use std::{collections::BTreeMap, path::Path, process::Command};

#[test]
fn invalid_policy() {
    for value in [
        json!({}),
        json!({"requires_backend_min":"1.x"}),
        json!({"requires_backend_min":"2.0.0","requires_backend_max":"1.0.0"}),
        json!({"requires_backend_min":"1.0.0","typo":"x"}),
    ] {
        let policy = serde_json::from_value::<Policy>(value);
        assert!(policy.is_err() || policy.unwrap().validate().is_err());
    }
}

pub(super) fn plan(
    dir: &Path,
    backend_version: &str,
    mcp_version: &str,
    incompatible: bool,
) -> std::path::PathBuf {
    let mut descriptors = BTreeMap::new();
    for (who, version) in [
        ("backend", backend_version),
        ("frontend", "0.2.1"),
        ("mcp", mcp_version),
    ] {
        let mut document = spec();
        if incompatible && who == "backend" {
            response(&mut document)["required"] = json!(["state"]);
        }
        let bundle = Bundle {
            format: "marreq.contract.v2".into(),
            component: who.into(),
            version: version.parse().unwrap(),
            source_sha: "a".repeat(40),
            variant: (who == "backend").then(|| "server".into()),
            compatibility: (who != "backend").then(|| Policy {
                requires_backend_min: "0.2.0".parse().unwrap(),
                requires_backend_max: Some("0.2.99".parse().unwrap()),
            }),
            frontend_compatibility: (who == "backend").then(|| FrontendRange {
                min_version: "0.2.0".parse().unwrap(),
                max_version: "0.3.0".parse().unwrap(),
            }),
            contract: document,
        };
        let filename = format!("{who}.json");
        contract::write(&dir.join(&filename), &bundle).unwrap();
        descriptors.insert(
            who,
            Artifact {
                file: filename.into(),
                sha256: artifact::hash(&std::fs::read(dir.join(format!("{who}.json"))).unwrap()),
                version: bundle.version,
                source_sha: bundle.source_sha,
            },
        );
    }
    let plan = Plan {
        format: "marreq.deployment-contract.v2".into(),
        variant: "server".into(),
        backend: descriptors.remove("backend").unwrap(),
        frontend: descriptors.remove("frontend").unwrap(),
        mcp: descriptors.remove("mcp").unwrap(),
    };
    let path = dir.join("plan.json");
    contract::write(&path, &plan).unwrap();
    path
}

#[test]
fn compatible_backend_only_and_mcp_only_updates() {
    let dir = tempfile::tempdir().unwrap();
    for (b, m) in [("0.2.4", "0.3.0"), ("0.2.3", "0.3.1")] {
        let path = plan(dir.path(), b, m, false);
        assert_eq!(artifact::verify(&path, None).unwrap()["compatible"], true);
    }
}

#[test]
fn incompatible_backend_only_update() {
    let dir = tempfile::tempdir().unwrap();
    assert!(artifact::verify(&plan(dir.path(), "0.2.4", "0.3.0", true), None).is_err());
}

#[test]
fn compatible_frontend_only_update() {
    let dir = tempfile::tempdir().unwrap();
    let path = plan(dir.path(), "0.2.4", "0.3.0", false);
    let bundle_path = dir.path().join("frontend.json");
    let mut bundle: Bundle = serde_json::from_value(contract::load(&bundle_path).unwrap()).unwrap();
    bundle.version = "0.2.2".parse().unwrap();
    contract::write(&bundle_path, &bundle).unwrap();
    let mut deployment: Plan = serde_json::from_value(contract::load(&path).unwrap()).unwrap();
    deployment.frontend.version = bundle.version;
    deployment.frontend.sha256 = artifact::hash(&std::fs::read(bundle_path).unwrap());
    contract::write(&path, &deployment).unwrap();
    assert_eq!(artifact::verify(&path, None).unwrap()["compatible"], true);
}

#[test]
fn missing_historical_artifact() {
    let dir = tempfile::tempdir().unwrap();
    let path = plan(dir.path(), "0.2.4", "0.3.0", false);
    std::fs::remove_file(dir.path().join("mcp.json")).unwrap();
    assert!(
        artifact::verify(&path, None)
            .unwrap_err()
            .to_string()
            .contains("missing historical")
    );
}

#[test]
fn descriptor_source_and_version_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    for key in ["source_sha", "version"] {
        let path = plan(dir.path(), "0.2.4", "0.3.0", false);
        let mut p = contract::load(&path).unwrap();
        p["mcp"][key] = json!(if key == "source_sha" {
            "b".repeat(40)
        } else {
            "0.3.9".into()
        });
        contract::write(&path, &p).unwrap();
        assert!(
            artifact::verify(&path, None)
                .unwrap_err()
                .to_string()
                .contains("mismatch")
        );
    }
}

#[test]
fn tampered_contract_hash() {
    let dir = tempfile::tempdir().unwrap();
    let path = plan(dir.path(), "0.2.4", "0.3.0", false);
    std::fs::write(dir.path().join("mcp.json"), "{}").unwrap();
    assert!(
        artifact::verify(&path, None)
            .unwrap_err()
            .to_string()
            .contains("SHA-256 mismatch")
    );
}

#[test]
fn candidate_identity_requires_exact_source_version_and_bundle() {
    let dir = tempfile::tempdir().unwrap();
    let path = plan(dir.path(), "0.2.4", "0.3.0", false);
    let bundle = dir.path().join("backend.json");
    let version = "0.2.4".parse().unwrap();
    let wrong_version = "0.2.5".parse().unwrap();
    let source = "a".repeat(40);
    artifact::verify(&path, Some(("backend", &version, &source, &bundle))).unwrap();
    for (version, source) in [(&wrong_version, source.clone()), (&version, "b".repeat(40))] {
        assert!(artifact::verify(&path, Some(("backend", version, &source, &bundle))).is_err());
    }
    let altered = dir.path().join("altered.json");
    let mut document = contract::load(&bundle).unwrap();
    document["contract"]["paths"] = json!({});
    contract::write(&altered, &document).unwrap();
    assert!(artifact::verify(&path, Some(("backend", &version, &source, &altered))).is_err());
}

#[test]
fn semver_build_metadata_does_not_change_policy_precedence() {
    Policy {
        requires_backend_min: "0.2.4".parse().unwrap(),
        requires_backend_max: Some("0.2.4".parse().unwrap()),
    }
    .accepts(&"0.2.4+source.sha".parse().unwrap())
    .unwrap();
}

#[test]
fn relative_plan_paths_work() {
    let dir = tempfile::tempdir().unwrap();
    plan(dir.path(), "0.2.4", "0.3.0", false);
    let result = Command::new(env!("CARGO_BIN_EXE_marreq-contracts"))
        .current_dir(dir.path())
        .args(["verify", "--manifest", "plan.json"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stdout)
    );
}

#[test]
fn consumer_bundle_requires_committed_build_configuration() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("docs/developer/openapi")).unwrap();
    std::fs::create_dir_all(root.join("frontend/src")).unwrap();
    contract::write(
        &root.join(marreq_contracts::source::DEFAULT_SPEC),
        &marreq_contracts::source::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/developer/openapi/root.yaml"),
        )
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("frontend/src/client.ts"),
        "fetch('/api/meta/build');",
    )
    .unwrap();
    std::fs::write(
        root.join("frontend/compatibility.json"),
        r#"{"requires_backend_min":"0.2.0","requires_backend_max":"0.2.99"}"#,
    )
    .unwrap();
    std::fs::write(root.join("frontend/vite.config.ts"), "export default {};").unwrap();
    for args in [
        vec!["init", "--quiet"],
        vec!["add", "."],
        vec![
            "-c",
            "user.name=Contract Test",
            "-c",
            "user.email=contract@example.test",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ],
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .unwrap()
                .success()
        );
    }
    let version = "0.2.1".parse().unwrap();
    let clean = artifact::export(root, "frontend", version, None).unwrap();
    assert_eq!(clean.source_sha.len(), 40);
    for relative in [
        "frontend/vite.config.ts",
        "docs/developer/openapi/new-domain.yaml",
        "scripts/bundle-openapi.mjs",
        "package.json",
        "package-lock.json",
    ] {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let original = std::fs::read(&path).ok();
        std::fs::write(&path, "uncommitted input").unwrap();
        let error = artifact::export(root, "frontend", "0.2.1".parse().unwrap(), None).unwrap_err();
        assert!(
            error.to_string().contains("commit source/contract inputs"),
            "{relative}: {error:#}"
        );
        if let Some(bytes) = original {
            std::fs::write(path, bytes).unwrap();
        } else {
            std::fs::remove_file(path).unwrap();
        }
    }
}

#[test]
fn inverted_backend_frontend_range_is_rejected() {
    let range = FrontendRange {
        min_version: "0.3.0".parse().unwrap(),
        max_version: "0.2.0".parse().unwrap(),
    };
    assert!(range.validate().is_err());
    assert!(range.accepts(&"0.2.1".parse().unwrap()).is_err());
}
