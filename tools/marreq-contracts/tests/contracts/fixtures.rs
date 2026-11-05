use serde_json::{Value, json};

pub(super) fn spec() -> Value {
    json!({"openapi":"3.1.0","info":{"title":"fixture","version":"1.0.0"},"paths":{"/api/things":{"post":{"operationId":"create","security":[{"cookie":[]}],"requestBody":{"required":true,"content":{"application/json":{"schema":{"type":"object","properties":{"title":{"type":"string"}},"required":["title"],"additionalProperties":false}}}},"responses":{"200":{"description":"OK","content":{"application/json":{"schema":{"type":"object","properties":{"id":{"type":"integer"},"state":{"type":"string","enum":["draft","approved"]}},"required":["id","state"],"additionalProperties":true}}}}}}}},"components":{"securitySchemes":{"cookie":{"type":"apiKey","in":"cookie","name":"session"}}}})
}

pub(super) fn request(s: &mut Value) -> &mut Value {
    &mut s["paths"]["/api/things"]["post"]["requestBody"]["content"]["application/json"]["schema"]
}

pub(super) fn response(s: &mut Value) -> &mut Value {
    &mut s["paths"]["/api/things"]["post"]["responses"]["200"]["content"]["application/json"]["schema"]
}
