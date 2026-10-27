// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compare the compiled Rocket mount inventory with the OpenAPI-derived artifact.
//! No database or test-double routing: declared_mounts is also used by production.
use std::collections::BTreeSet;

#[test]
fn compiled_server_and_cloud_routes_match_openapi_inventory() {
    let expected: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../docs/developer/api-contract-routes.json"
    ))
    .expect("generated route inventory");
    let parameter = regex::Regex::new(r"<([\w]+)(?:\.\.)?>").unwrap();
    for (variant, extra) in [
        ("server", Vec::new()),
        ("cloud", marreq_cloud::routes::routes()),
    ] {
        let mounts = marreq_core::app::declared_mounts(extra);
        let actual: BTreeSet<_> = [("/api", mounts.api), ("", mounts.root)]
            .into_iter()
            .flat_map(|(base, routes)| {
                let parameter = &parameter;
                routes.into_iter().filter_map(move |(_, route)| {
                    let path = route.uri.path().to_owned();
                    // Origin root UI and internal CSRF denial routes are not
                    // HTTP consumer APIs. OAuth routes under root are included.
                    if base.is_empty()
                        && !(path.starts_with("/oauth/")
                            || path.starts_with("/.well-known/")
                            || path.starts_with("/api/"))
                    {
                        return None;
                    }
                    let normalized = parameter.replace_all(&path, "{$1}");
                    Some((route.method.to_string(), format!("{base}{normalized}")))
                })
            })
            .collect();
        let documented: BTreeSet<_> = expected
            .iter()
            .filter(|op| {
                op["variants"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v == variant)
            })
            .map(|op| {
                (
                    op["method"].as_str().unwrap().to_owned(),
                    op["path"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        assert_eq!(actual, documented, "{variant} route contract drift");
    }
}
