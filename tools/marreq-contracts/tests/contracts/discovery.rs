use super::fixtures::spec;
use marreq_contracts::{contract, coverage};
use serde_json::json;

#[test]
fn shared_transport_does_not_hide_concrete_calls() {
    let ops = contract::operations(&spec(), None).unwrap();
    for source in [
        "function fetchJson(path:string){return fetch('/api/missing');}",
        "class Client {request(path:string){const url='/api/missing';return fetch(url);}}",
    ] {
        assert!(
            coverage::analyze(source, false, &ops, true).is_err(),
            "{source}"
        );
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
    assert!(contract::select(&doc, &coverage.operations, None).is_err());
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
        "client.open('POST', '/api/missing');",
        "new XMLHttpRequest();",
        "new EventSource(endpoint);",
        "new WebSocket(endpoint);",
        "navigator.sendBeacon(endpoint, data);",
        "import client from 'axios'; client.get(endpoint);",
        "import {request} from 'node:https';",
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
fn browser_calls_and_assignments_cannot_hide_dependencies() {
    let mut document = spec();
    document["paths"]["/api/things"]["get"] = json!({"responses":{"200":{"description":"ok"}}});
    let operations = contract::operations(&document, None).unwrap();
    for source in [
        "window.location.assign('/api/missing')",
        "window.open('/api/missing')",
        "window.location.href='/api/missing'",
        "window.location.href=unknown",
        "window.location.assign(unknown)",
    ] {
        assert!(
            coverage::analyze(source, false, &operations, false).is_err(),
            "{source}"
        );
    }
    for source in [
        "window.location.assign('/api/things')",
        "window.open('/api/things', '_blank')",
        "window.location.href='/api/things'",
    ] {
        assert_eq!(
            coverage::analyze(source, false, &operations, false)
                .unwrap()
                .operations
                .len(),
            1,
            "{source}"
        );
    }
    assert!(
        coverage::analyze(
            "const url=URL.createObjectURL(blob);window.location.href=url",
            false,
            &operations,
            false
        )
        .unwrap()
        .operations
        .is_empty()
    );
}
