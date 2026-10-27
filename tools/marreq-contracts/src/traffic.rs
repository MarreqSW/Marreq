//! Real request/response evidence collected by the established Rocket test.
use crate::{contract, coverage};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};
pub fn verify(spec: &Value, path: &Path) -> Result<Value> {
    let mut checked = 0;
    let mut unverified = 0;
    let mut seen = BTreeSet::new();
    for line in std::fs::read_to_string(path)?
        .lines()
        .filter(|l| !l.trim().is_empty())
    {
        let record: Value = serde_json::from_str(line)?;
        let mode = record["variant"].as_str().unwrap_or("server");
        crate::artifact::variant(mode)?;
        let operations = contract::operations(spec, Some(mode))?;
        let actual = record["path"]
            .as_str()
            .context("request path")?
            .split('?')
            .next()
            .unwrap();
        let method = record["method"]
            .as_str()
            .context("HTTP method")?
            .to_lowercase();
        let mut matches: Vec<_> = operations
            .iter()
            .filter(|(key, _)| {
                key.method == method && coverage::intersects(actual, &route_pattern(&key.path))
            })
            .collect();
        matches.sort_by_key(|(key, _)| key.path.matches('{').count());
        let (key, op) = matches
            .first()
            .context("actual HTTP call missing in declared contract")?;
        if op
            .get("x-marreq-unverified")
            .is_some_and(|v| v != &Value::Bool(false))
        {
            unverified += 1;
            continue;
        }
        let status = record["status"]
            .as_u64()
            .context("response status")?
            .to_string();
        let response = op["responses"]
            .get(&status)
            .or_else(|| op["responses"].get("default"))
            .with_context(|| format!("{key}: undocumented actual HTTP status {status}"))?;
        let response = if let Some(r) = response["$ref"].as_str() {
            spec.pointer(&r[1..]).context("response ref")?
        } else {
            response
        };
        if let Some(schema) = response.pointer("/content/application~1json/schema") {
            contract::validate_instance(spec, schema, &record["response"])?;
        }
        if status.starts_with('2') && !record["request"].is_null() {
            let schema = op
                .pointer("/requestBody/content/application~1json/schema")
                .with_context(|| format!("{key}: undeclared JSON body"))?;
            contract::validate_instance(spec, schema, &record["request"])?;
        }
        checked += 1;
        seen.insert((key.method.clone(), key.path.clone(), status));
    }
    for (method, path, status) in [
        ("get", "/api/meta/build", "200"),
        ("get", "/api/meta/deployment", "200"),
        ("post", "/api/projects/{project_id}/use-cases", "201"),
        (
            "put",
            "/api/projects/{project_id}/use-cases/{id}/draft",
            "200",
        ),
        ("get", "/api/projects/{project_id}/use-cases", "403"),
    ] {
        ensure!(
            seen.contains(&(method.into(), path.into(), status.into())),
            "critical real-backend evidence missing: {method} {path} {status}"
        );
    }
    Ok(json!({"validated_exchanges":checked,"unverified_exchanges":unverified}))
}
fn route_pattern(path: &str) -> String {
    let mut out = String::new();
    let mut param = false;
    for c in path.chars() {
        match c {
            '{' => {
                param = true;
                out.push('*');
            }
            '}' => param = false,
            _ if !param => out.push(c),
            _ => (),
        }
    }
    out
}
