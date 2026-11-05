use super::fixtures::spec;
use marreq_contracts::contract;
use serde_json::json;

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
fn changed_authorization_guard_fails() {
    let mut old = spec();
    old["paths"]["/api/things"]["post"]["x-marreq-authorization"] = json!("project-member");
    let mut new = old.clone();
    new["paths"]["/api/things"]["post"]["x-marreq-authorization"] = json!("instance-admin");
    assert!(contract::compare(&old, &new, "server").is_err());
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
fn changing_csrf_requirements_is_incompatible() {
    let mut old = spec();
    old["paths"]["/api/things"]["post"]["x-marreq-csrf"] = json!("session-csrf");
    let mut new = old.clone();
    new["paths"]["/api/things"]["post"]["x-marreq-csrf"] = json!("auth-origin-fallback");
    assert!(contract::compare(&old, &new, "server").is_err());
}
