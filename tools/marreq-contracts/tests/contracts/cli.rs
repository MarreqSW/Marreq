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
