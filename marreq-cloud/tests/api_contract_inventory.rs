// SPDX-License-Identifier: AGPL-3.0-or-later
//! Consumer contracts must correspond to compiled production Rocket mounts.
//! Additional, unconsumed backend routes deliberately do not fail this check.
use std::collections::BTreeSet;

fn spec() -> serde_json::Value {
    marreq_contracts::contract::load(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/developer/openapi.yaml"),
    )
    .unwrap()
}

#[test]
fn consumed_contract_routes_exist_in_server_and_cloud() {
    let spec = spec();
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

#[test]
fn audited_cache_models_match_the_wire_schemas() {
    let spec = spec();
    let cache = marreq_core::repository::cache::Cache::new(60);
    for (name, value) in [
        ("CacheStats", serde_json::to_value(cache.stats()).unwrap()),
        ("CacheRecommendations", cache.get_recommendations()),
    ] {
        let schema = &spec["components"]["schemas"][name];
        marreq_contracts::contract::validate_instance(&spec, schema, &value).unwrap();
        for property in schema["required"].as_array().unwrap() {
            let mut broken = value.clone();
            broken
                .as_object_mut()
                .unwrap()
                .remove(property.as_str().unwrap());
            assert!(marreq_contracts::contract::validate_instance(&spec, schema, &broken).is_err());
        }
    }
}

#[test]
fn audited_auth_requests_deserialize_as_the_real_rust_forms() {
    use marreq_core::models::{ChangePasswordForm, ProfileUpdate};
    use serde_json::json;
    let spec = spec();
    for (name, value) in [
        (
            "AuthProfileUpdate",
            json!({"name":"Manager","email":"manager@example.test"}),
        ),
        (
            "AuthProfileUpdate",
            json!({"name":"Manager","email":"manager@example.test","current_password":null}),
        ),
        (
            "AuthChangePassword",
            json!({"current_password":"old","new_password":"new","confirm_password":"new"}),
        ),
    ] {
        marreq_contracts::contract::validate_instance(
            &spec,
            &spec["components"]["schemas"][name],
            &value,
        )
        .unwrap();
        if name == "AuthProfileUpdate" {
            serde_json::from_value::<ProfileUpdate>(value).unwrap();
        } else {
            serde_json::from_value::<ChangePasswordForm>(value).unwrap();
        }
    }
}

#[test]
fn attachment_dtos_preserve_nullable_fields_and_storage_limits() {
    use marreq_core::api::attachments::{AttachmentView, StorageView};
    let spec = spec();
    let timestamp = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let attachment = AttachmentView {
        id: 1,
        entity_type: "requirement".into(),
        entity_id: 2,
        filename: "evidence.pdf".into(),
        content_type: "application/pdf".into(),
        size_bytes: 100,
        uploaded_by: None,
        uploaded_by_name: None,
        created_at: timestamp,
        deleted: false,
    };
    let storage = StorageView {
        used_bytes: 100,
        quota_bytes: 1024,
        quota_is_default: true,
        default_quota_bytes: 1024,
        retained_by_baselines_bytes: 0,
        max_file_bytes: 512,
        allowed_extensions: vec!["pdf"],
    };
    for (name, value) in [
        ("AttachmentView", serde_json::to_value(attachment).unwrap()),
        ("AttachmentStorage", serde_json::to_value(storage).unwrap()),
    ] {
        let schema = &spec["components"]["schemas"][name];
        marreq_contracts::contract::validate_instance(&spec, schema, &value).unwrap();
        let mut broken = value;
        let key = schema["required"][0].as_str().unwrap();
        broken.as_object_mut().unwrap().remove(key);
        assert!(marreq_contracts::contract::validate_instance(&spec, schema, &broken).is_err());
    }
}
