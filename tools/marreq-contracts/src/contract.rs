//! OpenAPI selection and validation. Standards parsing/comparison is delegated
//! to pinned oasdiff; JSON instances use the jsonschema crate in offline mode.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    process::Command,
};

pub const ENGINE_VERSION: &str = "1.34.0";
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Operation {
    pub path: String,
    pub method: String,
}
impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.method.to_uppercase(), self.path)
    }
}
pub fn load(path: &Path) -> Result<Value> {
    serde_yaml_ng::from_str(
        &std::fs::read_to_string(path)
            .with_context(|| format!("missing contract evidence: {}", path.display()))?,
    )
    .context("invalid JSON/YAML")
}
pub fn write(path: &Path, value: &impl Serialize) -> Result<()> {
    std::fs::write(path, serde_json::to_vec_pretty(value)?).context("write contract")
}
pub fn operations(spec: &Value, variant: Option<&str>) -> Result<BTreeMap<Operation, Value>> {
    let mut result = BTreeMap::new();
    for (path, item) in spec["paths"].as_object().context("OpenAPI paths missing")? {
        for (method, op) in item.as_object().context("invalid path item")? {
            if !["get", "post", "put", "patch", "delete", "head", "options"]
                .contains(&method.as_str())
            {
                continue;
            }
            if variant.is_some_and(|v| {
                op.get("x-marreq-variants")
                    .and_then(Value::as_array)
                    .is_some_and(|a| !a.iter().any(|x| x == v))
            }) {
                continue;
            }
            let mut op = op.clone();
            if let Some(parameters) = item.get("parameters") {
                let mut merged = parameters.as_array().context("path parameters")?.clone();
                merged.extend(op["parameters"].as_array().cloned().unwrap_or_default());
                op["parameters"] = json!(merged);
            }
            if op.get("security").is_none() {
                op["security"] = spec.get("security").cloned().unwrap_or(json!([]));
            }
            result.insert(
                Operation {
                    path: path.clone(),
                    method: method.clone(),
                },
                op,
            );
        }
    }
    Ok(result)
}
// Compact a consumer bundle to its operations and reachable local components.
// No dereferencing/schema comparison is implemented here.
pub fn select(
    spec: &Value,
    required: &BTreeSet<Operation>,
    variant: Option<&str>,
    verified: bool,
) -> Result<Value> {
    let available = operations(spec, variant)?;
    let mut selected = json!({"openapi":spec["openapi"],"info":spec["info"],"servers":spec.get("servers").cloned().unwrap_or(json!([{ "url":"/" }])),"paths":{},"components":{}});
    for key in required {
        let op = available.get(key).with_context(|| {
            format!(
                "required operation missing in {} backend: {key}",
                variant.unwrap_or("selected")
            )
        })?;
        if verified
            && op
                .get("x-marreq-unverified")
                .is_some_and(|v| v != &Value::Bool(false))
        {
            bail!("required consumer operation has unverified wire contract: {key}");
        }
        selected["paths"][&key.path][&key.method] = op.clone();
    }
    // Include just referenced components, including security schemes inherited
    // at the operation level. External refs are always rejected before tooling.
    loop {
        let mut references = BTreeSet::new();
        collect_refs(&selected, &mut references)?;
        for item in selected["paths"].as_object().unwrap().values() {
            for op in item.as_object().unwrap().values() {
                for req in op["security"].as_array().into_iter().flatten() {
                    for name in req.as_object().context("security requirement")?.keys() {
                        references.insert(format!("#/components/securitySchemes/{name}"));
                    }
                }
            }
        }
        let mut changed = false;
        for reference in references {
            let pointer = &reference[1..];
            if selected.pointer(pointer).is_none() {
                let parts: Vec<_> = pointer.trim_start_matches('/').split('/').collect();
                ensure!(
                    parts.len() == 3 && parts[0] == "components",
                    "unsupported non-component reference {reference}"
                );
                selected["components"][parts[1]][parts[2].replace("~1", "/").replace("~0", "~")] =
                    spec.pointer(pointer)
                        .with_context(|| format!("missing reference {reference}"))?
                        .clone();
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    Ok(selected)
}
fn collect_refs(value: &Value, out: &mut BTreeSet<String>) -> Result<()> {
    match value {
        Value::Object(map) => {
            if let Some(r) = map.get("$ref") {
                let r = r.as_str().context("invalid $ref")?;
                ensure!(
                    r.starts_with("#/"),
                    "external references are forbidden: {r}"
                );
                out.insert(r.into());
            }
            for v in map.values() {
                collect_refs(v, out)?;
            }
        }
        Value::Array(a) => {
            for v in a {
                collect_refs(v, out)?;
            }
        }
        _ => (),
    }
    Ok(())
}
fn engine() -> Result<Command> {
    let binary = std::env::var_os("OASDIFF").unwrap_or_else(|| "oasdiff".into());
    let output = Command::new(&binary).arg("--version").output().context(
        "install checksum-pinned oasdiff using scripts/install-oasdiff.sh (or set OASDIFF)",
    )?;
    let version = String::from_utf8_lossy(&output.stdout);
    ensure!(
        output.status.success()
            && version
                .split_whitespace()
                .any(|v| v.trim_start_matches('v') == ENGINE_VERSION),
        "expected oasdiff {ENGINE_VERSION}, got {version}"
    );
    Ok(Command::new(binary))
}
pub fn validate(spec: &Value) -> Result<()> {
    collect_refs(spec, &mut BTreeSet::new())?;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("contract.json");
    write(&path, spec)?;
    let output = engine()?
        .args([
            "validate",
            "--allow-external-refs=false",
            "--fail-on",
            "WARN",
            "--format",
            "json",
        ])
        .arg(path)
        .output()?;
    ensure!(
        output.status.success(),
        "OpenAPI validation failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
// Explicitly bound the schema dialect whose compatibility the pinned engine
// can establish. Advanced semantics are localized to selected consumer schemas,
// never silently ignored or applied to unrelated backend definitions.
fn schema_policy(schema: &Value) -> Result<()> {
    let Some(map) = schema.as_object() else {
        return Ok(());
    };
    let supported = [
        "$schema",
        "$ref",
        "$id",
        "$defs",
        "definitions",
        "type",
        "const",
        "enum",
        "allOf",
        "anyOf",
        "oneOf",
        "properties",
        "required",
        "additionalProperties",
        "items",
        "minItems",
        "maxItems",
        "uniqueItems",
        "minLength",
        "maxLength",
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "exclusiveMaximum",
        "multipleOf",
        "minProperties",
        "maxProperties",
        "pattern",
        "format",
        "title",
        "description",
        "default",
        "examples",
        "example",
        "deprecated",
        "readOnly",
        "writeOnly",
        "nullable",
        "discriminator",
        "externalDocs",
        "xml",
        "x-extensible-enum",
        "x-marreq-unverified",
    ];
    for key in map.keys() {
        ensure!(
            supported.contains(&key.as_str()),
            "schema keyword {key} is outside the supported compatibility dialect; affected operation needs explicit evidence"
        );
    }
    ensure!(
        !map.get("x-marreq-unverified")
            .is_some_and(|v| v != &Value::Bool(false)),
        "selected schema is unverified"
    );
    for key in ["properties", "$defs", "definitions"] {
        if let Some(values) = map.get(key).and_then(Value::as_object) {
            for value in values.values() {
                schema_policy(value)?;
            }
        }
    }
    for key in ["items", "additionalProperties"] {
        if let Some(value) = map.get(key) {
            schema_policy(value)?;
        }
    }
    for key in ["allOf", "anyOf", "oneOf"] {
        if let Some(values) = map.get(key).and_then(Value::as_array) {
            for value in values {
                schema_policy(value)?;
            }
        }
    }
    Ok(())
}
fn supported_schemas(document: &Value) -> Result<()> {
    if let Some(schemas) = document
        .pointer("/components/schemas")
        .and_then(Value::as_object)
    {
        for schema in schemas.values() {
            schema_policy(schema)?;
        }
    }
    fn inline(value: &Value) -> Result<()> {
        match value {
            Value::Object(map) => {
                for (key, value) in map {
                    if key == "schema" {
                        schema_policy(value)?;
                    } else if !matches!(key.as_str(), "example" | "examples" | "default") {
                        inline(value)?;
                    }
                }
            }
            Value::Array(values) => {
                for value in values {
                    inline(value)?;
                }
            }
            _ => (),
        }
        Ok(())
    }
    inline(document)
}

pub fn validate_wire_contract(document: &Value) -> Result<()> {
    validate(document)?;
    supported_schemas(document)
}

pub fn compare(consumer: &Value, backend: &Value, variant: &str) -> Result<Value> {
    crate::artifact::variant(variant)?;
    let required = operations(consumer, Some(variant))?.into_keys().collect();
    let old = select(consumer, &required, Some(variant), true)?;
    let new = select(backend, &required, Some(variant), true)?;
    validate_wire_contract(&old)?;
    validate_wire_contract(&new)?;
    // These are Marreq-specific authorization semantics not standard OpenAPI
    // security schemes. Compare per operation, never globally across the API.
    let a = operations(&old, None)?;
    let b = operations(&new, None)?;
    for (key, op) in &a {
        ensure!(
            op.get("x-marreq-csrf") == b[key].get("x-marreq-csrf"),
            "{key}: CSRF authentication requirement changed"
        );
        let scopes = |v: &Value| -> Result<BTreeSet<String>> {
            v.as_array()
                .context("scope declarations must be arrays")?
                .iter()
                .map(|s| Ok(s.as_str().context("scope must be string")?.to_owned()))
                .collect()
        };
        let old = op.get("x-marreq-bearer-scopes");
        let new = b[key].get("x-marreq-bearer-scopes");
        match (old, new) {
            (Some(old), Some(new)) => ensure!(
                scopes(new)?.is_subset(&scopes(old)?),
                "{key}: newly required Bearer scopes"
            ),
            (None, None) => (),
            _ => anyhow::bail!("{key}: Bearer authorization evidence changed"),
        }
    }
    let dir = tempfile::tempdir()?;
    let base = dir.path().join("consumer.json");
    let revision = dir.path().join("backend.json");
    write(&base, &old)?;
    write(&revision, &new)?;
    let output = engine()?
        .args([
            "breaking",
            "--allow-external-refs=false",
            "--fail-on",
            "WARN",
            "--format",
            "json",
            "--flatten-allof",
        ])
        .arg(base)
        .arg(revision)
        .output()?;
    let findings: Value = serde_json::from_slice(&output.stdout).unwrap_or(json!([]));
    ensure!(
        output.status.success(),
        "incompatible {variant} producer/consumer contract: {findings} {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(findings)
}
pub fn validate_instance(spec: &Value, schema: &Value, instance: &Value) -> Result<()> {
    let mut schema = schema.clone();
    if let Some(obj) = schema.as_object_mut() {
        obj.insert("components".into(), spec["components"].clone());
    }
    let validator = jsonschema::options()
        .offline()
        .build(&schema)
        .context("invalid/offline JSON schema")?;
    validator
        .validate(instance)
        .map_err(|e| anyhow::anyhow!("wire schema violation at {}: {}", e.instance_path(), e))
}
