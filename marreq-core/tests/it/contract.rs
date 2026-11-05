//! Bind the existing Rocket integration scenarios to the published wire contract.
//! No separate response fixtures or route inventory are maintained here.
use marreq_contracts::{contract, coverage};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::local::asynchronous::Client;
use rocket::{Build, Request, Response, Rocket};
use serde_json::Value;
use std::{io::Cursor, sync::OnceLock};

fn spec() -> &'static Value {
    static SPEC: OnceLock<Value> = OnceLock::new();
    SPEC.get_or_init(|| {
        marreq_contracts::source::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../docs/developer/openapi/root.yaml"),
        )
        .unwrap()
    })
}

fn operation(request: &Request<'_>) -> Option<Value> {
    let mut matches: Vec<_> = contract::operations(spec(), None)
        .unwrap()
        .into_iter()
        .filter(|(key, _)| {
            key.method.eq_ignore_ascii_case(request.method().as_str()) && {
                coverage::intersects(
                    request.uri().path().as_str(),
                    &coverage::route_pattern(&key.path),
                )
            }
        })
        .collect();
    matches.sort_by_key(|(key, _)| key.path.matches('{').count());
    matches
        .into_iter()
        .next()
        .map(|(_, op)| op)
        .filter(|op| op["x-marreq-unverified"] != true)
}

struct WireContract;

#[rocket::async_trait]
impl Fairing for WireContract {
    fn info(&self) -> Info {
        Info {
            name: "Consumer wire contract assertions",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let Some(op) = operation(request) else {
            return;
        };
        let status = response.status().code.to_string();
        let definition = op["responses"]
            .get(&status)
            .or_else(|| op["responses"].get("default"))
            .unwrap_or_else(|| {
                panic!(
                    "{} {}: undocumented HTTP {status}",
                    request.method(),
                    request.uri()
                )
            });
        let definition = definition["$ref"]
            .as_str()
            .map_or(definition, |r| spec().pointer(&r[1..]).unwrap());
        let success = (200..300).contains(&response.status().code);
        if success && let Some(headers) = definition["headers"].as_object() {
            for (name, header) in headers {
                if header["required"] == true {
                    assert!(
                        response.headers().contains(name),
                        "{}: missing {name}",
                        request.uri()
                    );
                }
            }
        }
        let media = response.content_type().map(|t| t.media_type().to_string());
        let declared = definition.get("content").and_then(Value::as_object);
        if success && let Some(declared) = declared {
            assert!(
                media.as_ref().is_some_and(|m| declared.contains_key(m))
                    || declared.contains_key("*/*"),
                "{} {}: undocumented successful media {media:?}",
                request.method(),
                request.uri()
            );
        }
        if media.as_deref() == Some("application/json")
            && let Some(schema) = definition.pointer("/content/application~1json/schema")
        {
            let bytes = response.body_mut().to_bytes().await.unwrap();
            let value: Value = serde_json::from_slice(&bytes).unwrap();
            contract::validate_instance(spec(), schema, &value).unwrap_or_else(|e| {
                panic!(
                    "{} {} HTTP {status}: {e:#}",
                    request.method(),
                    request.uri()
                )
            });
            response.set_sized_body(bytes.len(), Cursor::new(bytes));
        }
    }
}

pub async fn tracked(rocket: Rocket<Build>) -> Result<Client, Box<rocket::Error>> {
    Client::tracked(rocket.attach(WireContract))
        .await
        .map_err(Box::new)
}

#[rocket::get("/categories")]
fn incompatible_response() -> rocket::serde::json::Json<Value> {
    rocket::serde::json::Json(serde_json::json!([{"title":"missing guaranteed fields"}]))
}

#[rocket::async_test]
#[should_panic(expected = "wire schema violation")]
async fn response_assertions_reject_real_http_schema_drift() {
    let client = tracked(rocket::build().mount("/api", rocket::routes![incompatible_response]))
        .await
        .unwrap();
    client.get("/api/categories").dispatch().await;
}
