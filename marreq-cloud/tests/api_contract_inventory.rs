// SPDX-License-Identifier: AGPL-3.0-or-later
//! Consumer contracts must correspond to compiled production Rocket mounts.
//! Additional, unconsumed backend routes deliberately do not fail this check.
use std::collections::BTreeSet;

fn spec() -> &'static serde_json::Value {
    static SPEC: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    SPEC.get_or_init(|| {
        marreq_contracts::source::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../docs/developer/openapi/root.yaml"),
        )
        .unwrap()
    })
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
        let documented = marreq_contracts::contract::operations(spec, Some(variant)).unwrap();
        for key in documented.keys() {
            assert!(
                actual.contains(&(key.method.clone(), key.path.clone())),
                "{variant}: missing compiled {key}"
            );
        }
        for (key, op) in marreq_contracts::contract::operations(spec, None).unwrap() {
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
        marreq_contracts::contract::validate_instance(spec, schema, &value).unwrap();
        for property in schema["required"].as_array().unwrap() {
            let mut broken = value.clone();
            broken
                .as_object_mut()
                .unwrap()
                .remove(property.as_str().unwrap());
            assert!(marreq_contracts::contract::validate_instance(spec, schema, &broken).is_err());
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
            spec,
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
        marreq_contracts::contract::validate_instance(spec, schema, &value).unwrap();
        let mut broken = value;
        let key = schema["required"][0].as_str().unwrap();
        broken.as_object_mut().unwrap().remove(key);
        assert!(marreq_contracts::contract::validate_instance(spec, schema, &broken).is_err());
    }
}

fn request_model<T: serde::de::DeserializeOwned>(operation_id: &str, value: serde_json::Value) {
    let spec = spec();
    let op = marreq_contracts::contract::operations(spec, None)
        .unwrap()
        .into_values()
        .find(|op| op["operationId"] == operation_id)
        .unwrap();
    let schema = op
        .pointer("/requestBody/content/application~1json/schema")
        .unwrap();
    marreq_contracts::contract::validate_instance(spec, schema, &value).unwrap();
    assert!(
        serde_json::from_value::<T>(value.clone()).is_ok(),
        "{operation_id}: real Rust request rejected its fixture"
    );
    // The declared wire language must not accept missing/null/wrongly typed
    // fields which the actual handler's Serde type would reject.
    for key in value.as_object().unwrap().keys() {
        let mut omitted = value.clone();
        omitted.as_object_mut().unwrap().remove(key);
        let mut candidates = vec![omitted];
        for replacement in [
            serde_json::Value::Null,
            serde_json::json!(false),
            serde_json::json!(42),
            serde_json::json!({}),
            serde_json::json!([]),
        ] {
            let mut changed = value.clone();
            changed[key] = replacement;
            candidates.push(changed);
        }
        for candidate in candidates {
            if marreq_contracts::contract::validate_instance(spec, schema, &candidate).is_ok() {
                assert!(
                    serde_json::from_value::<T>(candidate).is_ok(),
                    "{operation_id}: contract accepts an invalid Rust field {key}"
                );
            }
        }
    }
}

#[test]
fn consumed_json_requests_match_real_handler_deserializers() {
    use marreq_core::{api, models};
    use serde_json::json;
    let tagged = json!({"title":"General","description":"Description","tag":"GEN","project_id":1});
    request_model::<models::NewCategory>("categories_create", tagged.clone());
    request_model::<models::NewApplicability>("applicability_create", tagged.clone());
    request_model::<models::NewVerificationMethod>(
        "verification_methods_create_by_project",
        tagged.clone(),
    );
    request_model::<models::NewRequirementStatus>(
        "status_create_requirement_status",
        tagged.clone(),
    );
    request_model::<models::NewVerificationStatus>(
        "verification_status_create_verification_status",
        tagged,
    );
    request_model::<models::UserCreateRequest>(
        "users_create",
        json!({"username":"user","name":"Name","email":"user@example.test","password":"test-only","is_admin":false}),
    );
    request_model::<models::UpdateUser>(
        "users_update",
        json!({"username":"user","name":"Name","email":"user@example.test","is_admin":false}),
    );
    request_model::<models::NewGroup>(
        "groups_create",
        json!({"name":"Group","description":null,"owner_id":null}),
    );
    request_model::<models::CustomFieldDefinitionPayload>(
        "custom_fields_create",
        json!({"label":"Field","field_type":"text","enum_values":null,"sort_order":null}),
    );
    request_model::<models::SavedViewPayload>(
        "saved_views_create",
        json!({"name":"View","visibility":"private","definition":{}}),
    );
    request_model::<api::projects::CreateProjectRequest>(
        "projects_create",
        json!({"name":"Project"}),
    );
    request_model::<api::projects::UpdateProjectRequest>(
        "projects_update",
        json!({"name":"Project","description":null,"status":"Active","owner_id":1,"group_id":null}),
    );
    request_model::<api::requirements::RequirementCreateRequest>(
        "requirements_create_by_project",
        json!({"title":"Title","description":"Description","author_id":1,"category_id":1,"status_id":1,"reference_code":"REQ-1","reviewer_id":1,"applicability_id":1,"project_id":1,"verification_method_ids":[1]}),
    );
    request_model::<api::requirements::RequirementPatch>(
        "requirements_patch_by_project",
        json!({"title":"Changed","status_id":1,"custom_fields":[]}),
    );
    request_model::<models::NewVerification>(
        "verifications_create_by_project",
        json!({"name":"Verification","reference_code":"VER-1","description":"Description","source":"manual","status_id":1,"project_id":1,"author_id":1,"reviewer_id":1}),
    );
    request_model::<api::comments::CreateCommentRequest>(
        "comments_create_by_project",
        json!({"body":"Comment","requirement_version_id":null}),
    );
    request_model::<api::baselines::CreateBaselineRequest>(
        "baselines_create",
        json!({"name":"Baseline","saved_view_id":null}),
    );
    request_model::<api::mcp::McpAuditRequest>(
        "mcp_internal_audit",
        json!({"tool_name":"list_projects","is_write":false}),
    );
}

#[test]
fn report_defaults_and_section_catalog_match_the_contract() {
    use marreq_core::reports::{
        definition::{DefaultContext, ReportDefinition, ReportType},
        sections,
    };
    let spec = spec();
    let context = DefaultContext {
        project_name: "Project".into(),
        project_slug: "project".into(),
        user_name: "Tester".into(),
        today: "2026-11-03".into(),
    };
    for kind in ReportType::ALL {
        let value = serde_json::to_value(ReportDefinition::default_for(kind, &context)).unwrap();
        for name in ["ReportDefinition", "ReportDefinitionInput"] {
            marreq_contracts::contract::validate_instance(
                spec,
                &spec["components"]["schemas"][name],
                &value,
            )
            .unwrap();
        }
    }
    for section in sections::catalog() {
        let value = serde_json::to_value(section).unwrap();
        marreq_contracts::contract::validate_instance(
            spec,
            &spec["components"]["schemas"]["ReportSectionSpec"],
            &value,
        )
        .unwrap();
    }
}
