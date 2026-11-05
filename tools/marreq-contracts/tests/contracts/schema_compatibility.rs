use super::fixtures::{request, response, spec};
use marreq_contracts::contract::{self, Operation};
use serde_json::json;
use std::collections::BTreeSet;

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
fn multipart_required_field_changes_use_the_standards_engine() {
    let mut old = spec();
    let schema = request(&mut old).clone();
    old["paths"]["/api/things"]["post"]["requestBody"]["content"] =
        json!({"multipart/form-data":{"schema":schema}});
    let mut new = old.clone();
    let schema = &mut new["paths"]["/api/things"]["post"]["requestBody"]["content"]["multipart/form-data"]
        ["schema"];
    schema["properties"]["new_field"] = json!({"type":"string"});
    schema["required"] = json!(["title", "new_field"]);
    assert!(contract::compare(&old, &new, "server").is_err());
}

#[test]
fn file_download_media_type_removal_fails() {
    let mut old = spec();
    old["paths"]["/api/things"]["post"]["responses"]["200"]["content"] =
        json!({"application/pdf":{"schema":{"type":"string","format":"binary"}}});
    let mut new = old.clone();
    new["paths"]["/api/things"]["post"]["responses"]["200"]["content"] =
        json!({"application/json":{"schema":{"type":"object"}}});
    assert!(contract::compare(&old, &new, "server").is_err());
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
    let consumer = contract::select(&old, &required, None).unwrap();
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
