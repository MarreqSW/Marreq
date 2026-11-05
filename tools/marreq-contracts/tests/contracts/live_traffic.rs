use super::fixtures::spec;
use serde_json::{Value, json};

#[test]
fn traffic_requires_both_real_backend_variants() {
    let mut document = spec();
    document["paths"] = json!({});
    let targets = [
        ("get", "/api/meta/build", 200),
        ("get", "/api/meta/deployment", 200),
        ("post", "/api/projects/{project_id}/use-cases", 201),
        (
            "put",
            "/api/projects/{project_id}/use-cases/{id}/draft",
            200,
        ),
        ("get", "/api/projects/{project_id}/use-cases", 403),
    ];
    let mut records = vec![];
    for (method, path, status) in targets {
        document["paths"][path][method] = json!({"responses":{status.to_string():{"description":"fixture","content":{"application/json":{"schema":{"type":"object","required":["id"],"properties":{"id":{"type":"integer"}}}}}}}});
        for variant in ["server", "cloud"] {
            records.push(json!({"variant":variant,"method":method,"path":path.replace("{project_id}","1").replace("{id}","2"),"status":status,"response":{"id":1}}));
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("traffic.jsonl");
    let write = |records: &[Value]| {
        std::fs::write(
            &path,
            records
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap()
    };
    write(&records);
    marreq_contracts::traffic::verify(&document, &path).unwrap();
    let server_only: Vec<_> = records
        .iter()
        .filter(|r| r["variant"] == "server")
        .cloned()
        .collect();
    write(&server_only);
    assert!(
        marreq_contracts::traffic::verify(&document, &path)
            .unwrap_err()
            .to_string()
            .contains("cloud real-backend evidence missing")
    );
    records[0]["response"]["id"] = json!("wrong type");
    write(&records);
    assert!(marreq_contracts::traffic::verify(&document, &path).is_err());
}
