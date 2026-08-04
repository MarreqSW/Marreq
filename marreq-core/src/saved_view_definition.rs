//! Validate and normalize saved view definition JSON (issue #110).

use crate::repository::errors::RepoError;
use serde_json::{json, Value};

const ALLOWED_VIEW_MODES: &[&str] = &["table", "list"];
const ALLOWED_SORT_DIRS: &[&str] = &["asc", "desc"];

/// Parsed filter/sort fields from a saved view definition.
#[derive(Debug, Clone, Default)]
pub struct AppliedViewFilters {
    pub status_id: Option<i32>,
    pub category_id: Option<i32>,
    pub approval_state: Option<String>,
    pub q: Option<String>,
    pub sort_column: Option<String>,
    pub sort_dir: String,
}

pub fn filters_from_definition(definition: &Value) -> AppliedViewFilters {
    let mut out = AppliedViewFilters {
        sort_dir: "asc".into(),
        ..Default::default()
    };
    let Some(obj) = definition.as_object() else {
        return out;
    };
    if let Some(filters) = obj.get("filters").and_then(|v| v.as_object()) {
        out.status_id = filters
            .get("status_id")
            .and_then(|v| v.as_i64())
            .map(|n| n as i32);
        out.category_id = filters
            .get("category_id")
            .and_then(|v| v.as_i64())
            .map(|n| n as i32);
        out.approval_state = filters
            .get("approval_state")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        out.q = filters
            .get("q")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .filter(|s| !s.trim().is_empty());
    }
    if let Some(sort) = obj.get("sort").and_then(|v| v.as_object()) {
        out.sort_column = sort
            .get("column")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        if let Some(dir) = sort.get("dir").and_then(|v| v.as_str()) {
            out.sort_dir = dir.to_string();
        }
    }
    out
}

/// Keep requirements matching view filters (status / category / approval / q).
pub fn filter_requirements_by_definition(
    requirements: Vec<crate::models::Requirement>,
    definition: &Value,
) -> Vec<crate::models::Requirement> {
    let applied = filters_from_definition(definition);
    let mut requirements = requirements;
    if let Some(sid) = applied.status_id {
        requirements.retain(|r| r.status_id == sid);
    }
    if let Some(cid) = applied.category_id {
        requirements.retain(|r| r.category_id == cid);
    }
    if let Some(state) = applied.approval_state.as_deref() {
        let lower = state.to_lowercase();
        requirements.retain(|r| r.approval_state.to_lowercase() == lower);
    }
    if let Some(raw_q) = applied.q.as_deref() {
        let needle = raw_q.trim().to_lowercase();
        if !needle.is_empty() {
            requirements.retain(|r| {
                [
                    r.reference_code.as_str(),
                    r.title.as_str(),
                    r.description.as_str(),
                ]
                .join(" ")
                .to_lowercase()
                .contains(&needle)
            });
        }
    }
    requirements
}

/// Validate `definition` for create/update. Returns a normalized object.
pub fn validate_saved_view_definition(definition: &Value) -> Result<Value, RepoError> {
    let obj = definition
        .as_object()
        .ok_or_else(|| RepoError::BadInput("definition must be a JSON object".into()))?;

    let version = obj.get("version").and_then(|v| v.as_i64()).unwrap_or(1);
    if version != 1 {
        return Err(RepoError::BadInput("definition.version must be 1".into()));
    }

    let entity = obj
        .get("entity")
        .and_then(|v| v.as_str())
        .unwrap_or("requirements");
    if entity != "requirements" {
        return Err(RepoError::BadInput(
            "definition.entity must be \"requirements\"".into(),
        ));
    }

    let filters = obj.get("filters").cloned().unwrap_or_else(|| json!({}));
    if !filters.is_object() {
        return Err(RepoError::BadInput(
            "definition.filters must be an object".into(),
        ));
    }

    let sort = obj
        .get("sort")
        .cloned()
        .unwrap_or_else(|| json!({ "column": null, "dir": "asc" }));
    if let Some(dir) = sort.get("dir").and_then(|v| v.as_str()) {
        if !ALLOWED_SORT_DIRS.contains(&dir) {
            return Err(RepoError::BadInput(
                "definition.sort.dir must be asc or desc".into(),
            ));
        }
    }

    let columns = obj.get("columns").cloned().unwrap_or(Value::Null);
    if !(columns.is_null() || columns.is_array()) {
        return Err(RepoError::BadInput(
            "definition.columns must be null or an array".into(),
        ));
    }

    let mut ui = obj
        .get("ui")
        .cloned()
        .unwrap_or_else(|| json!({ "view_mode": "table", "page_size": 25 }));
    if let Some(ui_obj) = ui.as_object_mut() {
        let mode = ui_obj
            .get("view_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("table");
        if !ALLOWED_VIEW_MODES.contains(&mode) {
            return Err(RepoError::BadInput(
                "definition.ui.view_mode must be table or list".into(),
            ));
        }
        let page_size = ui_obj
            .get("page_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(25)
            .clamp(1, 500);
        ui_obj.insert("view_mode".into(), json!(mode));
        ui_obj.insert("page_size".into(), json!(page_size));
    } else {
        return Err(RepoError::BadInput(
            "definition.ui must be an object".into(),
        ));
    }

    Ok(json!({
        "version": 1,
        "entity": "requirements",
        "filters": filters,
        "sort": sort,
        "columns": columns,
        "ui": ui,
    }))
}

pub fn validate_visibility(visibility: &str) -> Result<String, RepoError> {
    match visibility.trim().to_lowercase().as_str() {
        "private" => Ok("private".into()),
        "shared" => Ok("shared".into()),
        _ => Err(RepoError::BadInput(
            "visibility must be private or shared".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_minimal_definition() {
        let v = validate_saved_view_definition(&json!({})).unwrap();
        assert_eq!(v["version"], 1);
        assert_eq!(v["entity"], "requirements");
    }

    #[test]
    fn rejects_bad_entity() {
        let err =
            validate_saved_view_definition(&json!({ "entity": "verifications" })).unwrap_err();
        assert!(matches!(err, RepoError::BadInput(_)));
    }
}
