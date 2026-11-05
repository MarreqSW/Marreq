use super::artifacts::plan;
use serde_json::Value;
use std::process::Command;

#[test]
fn cli_exit_codes_and_json_diagnostics() {
    let dir = tempfile::tempdir().unwrap();
    for bad in [false, true] {
        let path = plan(dir.path(), "0.2.4", "0.3.0", bad);
        let result = Command::new(env!("CARGO_BIN_EXE_marreq-contracts"))
            .arg("verify")
            .arg("--manifest")
            .arg(path)
            .output()
            .unwrap();
        assert_eq!(result.status.success(), !bad);
        let body: Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(body["compatible"], !bad);
    }
}

#[test]
fn type_selection_rejects_invalid_source_documents_before_discovery() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("root.yaml");
    let mut document = super::fixtures::spec();
    document["info"]["title"] = Value::Null;
    marreq_contracts::contract::write(&path, &document).unwrap();
    let output = dir.path().join("selected.json");
    let result = Command::new(env!("CARGO_BIN_EXE_marreq-contracts"))
        .args(["select", "--component", "frontend", "--spec"])
        .arg(path)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(!output.exists());
    let body: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(
        body.to_string().contains("OpenAPI validation failed"),
        "{body}"
    );
}
