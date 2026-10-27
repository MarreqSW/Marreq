use marreq_contracts::{
    artifact::{self, Artifact, Bundle, FrontendRange, Plan, Policy},
    contract::{self, Operation},
    coverage,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    process::Command,
};
fn spec() -> Value {
    json!({"openapi":"3.1.0","info":{"title":"fixture","version":"1.0.0"},"paths":{"/api/things":{"post":{"operationId":"create","security":[{"cookie":[]}],"requestBody":{"required":true,"content":{"application/json":{"schema":{"type":"object","properties":{"title":{"type":"string"}},"required":["title"],"additionalProperties":false}}}},"responses":{"200":{"description":"OK","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"},"state":{"type":"string","enum":["draft","approved"]}},"required":["id","state"],"additionalProperties":true}}}}}}}},"components":{"securitySchemes":{"cookie":{"type":"apiKey","in":"cookie","name":"session"}}}})
}
fn request(s: &mut Value) -> &mut Value {
    &mut s["paths"]["/api/things"]["post"]["requestBody"]["content"]["application/json"]["schema"]
}
fn response(s: &mut Value) -> &mut Value {
    &mut s["paths"]["/api/things"]["post"]["responses"]["200"]["content"]["application/json"]["schema"]
}
#[test]
fn valid_openapi() {
    contract::validate(&spec()).unwrap();
}
#[test]
fn missing_endpoint() {
    let mut new = spec();
    new["paths"] = json!({});
    assert!(contract::compare(&spec(), &new, "server").is_err());
}
#[test]
fn required_response_removed() {
    let mut new = spec();
    response(&mut new)["required"] = json!(["state"]);
    assert!(contract::compare(&spec(), &new, "server").is_err());
}
#[test]
fn response_type_changed() {
    let mut new = spec();
    response(&mut new)["properties"]["id"] = json!({"type":"string"});
    assert!(contract::compare(&spec(), &new, "server").is_err());
}
#[test]
fn new_required_request() {
    let mut new = spec();
    request(&mut new)["properties"]["reason"] = json!({"type":"string"});
    request(&mut new)["required"] = json!(["title", "reason"]);
    assert!(contract::compare(&spec(), &new, "server").is_err());
}
#[test]
fn additive_request_and_response() {
    let mut new = spec();
    request(&mut new)["properties"]["optional"] = json!({"type":"integer"});
    response(&mut new)["properties"]["extra"] = json!({"type":"boolean"});
    contract::compare(&spec(), &new, "server").unwrap();
}
#[test]
fn response_enum_extension() {
    let mut new = spec();
    response(&mut new)["properties"]["state"]["enum"] = json!(["draft", "approved", "reviewed"]);
    assert!(contract::compare(&spec(), &new, "cloud").is_err());
}
#[test]
fn request_enum_restriction() {
    let mut old = spec();
    let mut new = spec();
    request(&mut old)["properties"]["title"]["enum"] = json!(["a", "b"]);
    request(&mut new)["properties"]["title"]["enum"] = json!(["a"]);
    assert!(contract::compare(&old, &new, "cloud").is_err());
}
#[test]
fn authentication_requirement() {
    let mut new = spec();
    new["components"]["securitySchemes"]["bearer"] = json!({"type":"http","scheme":"bearer"});
    new["paths"]["/api/things"]["post"]["security"] = json!([{"bearer":[]}]);
    assert!(contract::compare(&spec(), &new, "server").is_err());
}
#[test]
fn unrelated_security_scheme_is_not_global_coupling() {
    let mut new = spec();
    new["components"]["securitySchemes"]["unrelated"] = json!({"type":"http","scheme":"bearer"});
    contract::compare(&spec(), &new, "server").unwrap();
}
#[test]
fn per_operation_scope_change() {
    let mut new = spec();
    new["paths"]["/api/things"]["post"]["x-marreq-bearer-scopes"] = json!(["things:write"]);
    assert!(contract::compare(&spec(), &new, "server").is_err());
}
#[test]
fn unrelated_backend_endpoint_can_change() {
    let mut old = spec();
    old["paths"]["/api/unrelated"] = json!({"get":{"responses":{"200":{"description":"old"}}}});
    let mut new = old.clone();
    new["paths"]["/api/unrelated"] =
        json!({"delete":{"responses":{"204":{"description":"different"}}}});
    let required = BTreeSet::from([Operation {
        method: "post".into(),
        path: "/api/things".into(),
    }]);
    let consumer = contract::select(&old, &required, None, true).unwrap();
    contract::compare(&consumer, &new, "server").unwrap();
}
#[test]
fn cloud_operation_not_required_on_server() {
    let mut consumer = spec();
    consumer["paths"]["/api/register"] =
        json!({"post":{"x-marreq-variants":["cloud"],"responses":{"200":{"description":"ok"}}}});
    contract::compare(&consumer, &spec(), "server").unwrap();
    assert!(contract::compare(&consumer, &spec(), "cloud").is_err());
}
#[test]
fn required_unverified_operation_fails() {
    let mut new = spec();
    new["paths"]["/api/things"]["post"]["x-marreq-unverified"] = json!(true);
    assert!(contract::compare(&spec(), &new, "server").is_err());
}
#[test]
fn external_refs_fail_without_network() {
    let mut new = spec();
    response(&mut new)["properties"]["id"] = json!({"$ref":"https://example.test/schema"});
    assert!(
        contract::validate(&new)
            .unwrap_err()
            .to_string()
            .contains("external references")
    );
}
#[test]
fn actual_json_instances_are_checked() {
    let schema = response(&mut spec()).clone();
    contract::validate_instance(&spec(), &schema, &json!({"id":1,"state":"draft"})).unwrap();
    assert!(contract::validate_instance(&spec(), &schema, &json!({"state":"draft"})).is_err());
    assert!(
        contract::validate_instance(&spec(), &schema, &json!({"id":"wrong","state":"draft"}))
            .is_err()
    );
}
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
fn plan(
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
fn type_script_grammar_resolves_helpers_and_union_methods() {
    let mut document = spec();
    document["paths"]["/api/projects/{project_id}/things/{id}"] = json!({"get":{"responses":{"200":{"description":"ok"}}},"delete":{"responses":{"204":{"description":"ok"}}}});
    let ops = contract::operations(&document, None).unwrap();
    let source = "const base = (id: number) => `/api/projects/${id}/things`; function read(id: number) { return fetchJson(`${base(1)}/${id}`, {method: remove ? 'DELETE' : 'GET'}); }";
    let coverage = coverage::analyze(source, false, &ops, false).unwrap();
    assert_eq!(coverage.operations.len(), 2);
}
#[test]
fn new_unmapped_call_cannot_bypass_coverage() {
    let ops = contract::operations(&spec(), None).unwrap();
    assert!(coverage::analyze("fetch('/api/new-api')", false, &ops, false).is_err());
    assert!(coverage::analyze("fetch(url)", false, &ops, false).is_err());
}
#[test]
fn dynamic_unverified_dependency_is_rejected() {
    let mut doc = spec();
    doc["paths"]["/api/things"]["post"]["x-marreq-unverified"] = json!(true);
    let coverage = coverage::analyze(
        "fetchJson('/api/things', {method:'POST'})",
        false,
        &contract::operations(&doc, None).unwrap(),
        false,
    )
    .unwrap();
    assert!(contract::select(&doc, &coverage.operations, None, true).is_err());
}
#[test]
fn segment_globs_do_not_capture_unrelated_children() {
    assert!(!coverage::intersects(
        "/api/things/*",
        "/api/things/*/versions"
    ));
    assert!(coverage::intersects("/api/things/*.pdf", "/api/things/*"));
}

#[test]
fn await_generic_calls_are_dependencies() {
    let coverage = coverage::analyze(
        "async function f(){ return await fetchJson<Row>('/api/things',{method:'POST'}); }",
        false,
        &contract::operations(&spec(), None).unwrap(),
        false,
    )
    .unwrap();
    assert_eq!(coverage.call_sites, 1);
    assert_eq!(coverage.operations.len(), 1);
}
#[test]
fn browser_navigation_is_a_dependency() {
    let mut document = spec();
    document["paths"]["/api/things"]["get"] = json!({"responses":{"200":{"description":"ok"}}});
    let ops = contract::operations(&document, None).unwrap();
    assert_eq!(
        coverage::analyze(
            "const a=<a href=\"/api/things\">Reports & exports</a>",
            true,
            &ops,
            false
        )
        .unwrap()
        .operations
        .len(),
        1
    );
    assert!(
        coverage::analyze(
            "const a=<a href=\"/api/missing\">link</a>",
            true,
            &ops,
            false
        )
        .is_err()
    );
}
#[test]
fn unsupported_schema_semantics_fail_locally() {
    let mut new = spec();
    response(&mut new)["unevaluatedProperties"] = json!(false);
    assert!(
        contract::compare(&spec(), &new, "server")
            .unwrap_err()
            .to_string()
            .contains("unevaluatedProperties")
    );
}
#[test]
fn fewer_required_scopes_are_compatible() {
    let mut old = spec();
    old["paths"]["/api/things"]["post"]["x-marreq-bearer-scopes"] =
        json!(["things:read", "things:write"]);
    let mut new = old.clone();
    new["paths"]["/api/things"]["post"]["x-marreq-bearer-scopes"] = json!(["things:read"]);
    contract::compare(&old, &new, "server").unwrap();
}

#[test]
fn numeric_resource_ids_do_not_require_reserved_siblings() {
    let mut document = spec();
    document["paths"]["/api/things/{id}"] =
        json!({"get":{"responses":{"200":{"description":"ok"}}}});
    document["paths"]["/api/things/internal_stats"] =
        json!({"get":{"responses":{"200":{"description":"unused"}}}});
    let coverage = coverage::analyze(
        "function get(id:number){return fetchJson(`/api/things/${id}`);}",
        false,
        &contract::operations(&document, None).unwrap(),
        false,
    )
    .unwrap();
    assert_eq!(coverage.operations.len(), 1);
    assert!(coverage::intersects("/api/things/#", "/api/things/123"));
    assert!(!coverage::intersects(
        "/api/things/#",
        "/api/things/internal_stats"
    ));
}

#[test]
fn aliases_and_unrecognized_clients_cannot_hide_new_calls() {
    let ops = contract::operations(&spec(), None).unwrap();
    for source in [
        "const f=fetch; f('/api/missing');",
        "import {fetchJson as api} from './transport'; api('/api/missing');",
        "axios.get('/api/missing');",
        "axios({url:'/api/missing'});",
    ] {
        assert!(
            coverage::analyze(source, false, &ops, false).is_err(),
            "{source}"
        );
    }
    assert!(coverage::analyze("function f(url:string){if(true){const url='/api/things';}return fetchJson(url,{method:'POST'});}",false,&ops,false).is_err());
    assert!(
        coverage::analyze(
            "let path='/api/things';path=external;fetchJson(path,{method:'POST'});",
            false,
            &ops,
            false
        )
        .is_err()
    );
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
fn unknown_schema_keywords_are_not_silently_accepted() {
    let mut new = spec();
    response(&mut new)["customLimit"] = json!(1);
    assert!(
        contract::compare(&spec(), &new, "server")
            .unwrap_err()
            .to_string()
            .contains("customLimit")
    );
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
