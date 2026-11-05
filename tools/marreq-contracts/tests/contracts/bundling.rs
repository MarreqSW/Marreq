use marreq_contracts::{contract, source};
use serde_json::json;
use std::{fs, path::Path};

fn fixture(root: &Path, reference: &str) -> std::path::PathBuf {
    fs::write(root.join("root.yaml"), format!(
        "openapi: 3.1.0\ninfo: {{title: fixture, version: 1.0.0}}\npaths:\n  /api/example: {{$ref: '{reference}'}}\n"
    )).unwrap();
    root.join("root.yaml")
}

#[test]
fn deterministic_local_bundle_preserves_wire_and_security_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let path = fixture(dir.path(), "./paths.yaml#/example");
    fs::write(
        dir.path().join("paths.yaml"),
        r#"
example:
  post:
    operationId: example
    security: [{bearer: [requirements:write]}]
    x-marreq-variants: [cloud]
    x-marreq-bearer-scopes: [requirements:write]
    x-marreq-csrf: session-csrf
    requestBody:
      content:
        application/json:
          schema: {$ref: './schemas.yaml#/Request'}
    responses:
      '201':
        description: Created
        content:
          application/json:
            schema: {$ref: './schemas.yaml#/Response'}
"#,
    )
    .unwrap();
    fs::write(
        dir.path().join("schemas.yaml"),
        r#"
Request: {type: object, properties: {name: {type: string}}, required: [name]}
Response: {type: object, properties: {id: {type: integer}}, required: [id]}
"#,
    )
    .unwrap();
    let first = source::load(&path).unwrap();
    let second = source::load(&path).unwrap();
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );
    let op = &first["paths"]["/api/example"]["post"];
    assert_eq!(op["security"], json!([{"bearer":["requirements:write"]}]));
    assert_eq!(op["x-marreq-variants"], json!(["cloud"]));
    assert_eq!(op["x-marreq-bearer-scopes"], json!(["requirements:write"]));
    assert_eq!(op["x-marreq-csrf"], "session-csrf");
    let schema = &first["components"]["schemas"]["Request"];
    contract::validate_instance(&first, schema, &json!({"name":"accepted"})).unwrap();
    assert!(contract::validate_instance(&first, schema, &json!({})).is_err());
    assert_eq!(
        first["components"]["schemas"]["Response"]["required"],
        json!(["id"])
    );
}

#[test]
fn unresolved_references_fail_instead_of_producing_partial_contracts() {
    for reference in ["./missing.yaml#/Thing", "./schemas.yaml#/Missing"] {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("schemas.yaml"), "Thing: {type: string}\n").unwrap();
        assert!(source::load(&fixture(dir.path(), reference)).is_err());
    }
}

#[test]
fn circular_reference_chains_are_rejected_by_the_standards_bundler() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.yaml"), "$ref: './b.yaml'\n").unwrap();
    fs::write(dir.path().join("b.yaml"), "$ref: './a.yaml'\n").unwrap();
    let error = source::load(&fixture(dir.path(), "./a.yaml")).unwrap_err();
    assert!(error.to_string().contains("circular"));
}

#[test]
fn network_and_directory_escape_references_are_rejected_offline() {
    for reference in [
        "https://example.invalid/api.yaml",
        "file:///etc/passwd",
        "../outside.yaml",
    ] {
        let dir = tempfile::tempdir().unwrap();
        assert!(source::load(&fixture(dir.path(), reference)).is_err());
    }
    #[cfg(unix)]
    {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("contract");
        fs::create_dir(&root).unwrap();
        fs::write(parent.path().join("outside.yaml"), "get: {responses: {}}\n").unwrap();
        std::os::unix::fs::symlink(parent.path().join("outside.yaml"), root.join("escape.yaml"))
            .unwrap();
        let error = source::load(&fixture(&root, "./escape.yaml")).unwrap_err();
        assert!(error.to_string().contains("escapes contract directory"));
    }
}
