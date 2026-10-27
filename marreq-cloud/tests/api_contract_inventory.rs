// SPDX-License-Identifier: AGPL-3.0-or-later
//! Consumer contracts must correspond to compiled production Rocket mounts.
//! Additional, unconsumed backend routes deliberately do not fail this check.
use std::collections::BTreeSet;

#[test]
fn consumed_contract_routes_exist_in_server_and_cloud() {
    let spec = marreq_contracts::contract::load(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/developer/openapi.yaml"),
    )
    .unwrap();
    for (variant, extra) in [
        ("server", Vec::new()),
        ("cloud", marreq_cloud::routes::routes()),
    ] {
        let mounts = marreq_core::app::declared_mounts(extra);
        let actual: BTreeSet<_> = [("/api", mounts.api), ("", mounts.root)]
            .into_iter()
            .flat_map(|(base, routes)| {
                routes.into_iter().map(move |(_, route)| {
                    (
                        route.method.to_string().to_lowercase(),
                        format!(
                            "{base}{}",
                            route.uri.path().replace('<', "{").replace('>', "}")
                        ),
                    )
                })
            })
            .collect();
        let documented = marreq_contracts::contract::operations(&spec, Some(variant)).unwrap();
        for key in documented.keys() {
            assert!(
                actual.contains(&(key.method.clone(), key.path.clone())),
                "{variant}: missing compiled {key}"
            );
        }
        for (key, op) in marreq_contracts::contract::operations(&spec, None).unwrap() {
            if op
                .get("x-marreq-variants")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|a| !a.iter().any(|x| x == variant))
            {
                assert!(
                    !actual.contains(&(key.method, key.path)),
                    "deployment-specific endpoint leaked into {variant}"
                );
            }
        }
    }
}
