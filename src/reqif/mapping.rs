//! Default ReqIF attribute long-name to ReqMan field mapping.

use std::collections::HashMap;

/// Returns default mapping: ReqIF attribute long-name (case-insensitive match) -> ReqMan field name.
pub fn default_attribute_mapping() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("Title".to_string(), "title".to_string());
    m.insert("Identifier".to_string(), "reference_code".to_string());
    m.insert("ReqId".to_string(), "reference_code".to_string());
    m.insert("Statement".to_string(), "description".to_string());
    m.insert("Description".to_string(), "description".to_string());
    m.insert("Status".to_string(), "status".to_string());
    m.insert("Rationale".to_string(), "justification".to_string());
    m
}

/// Resolve attribute value by trying known long-names (Title, Identifier, Statement, etc.).
pub fn get_attr(obj_attrs: &HashMap<String, String>, field: &str) -> Option<String> {
    let keys: Vec<&str> = match field {
        "title" => vec!["Title", "title"],
        "reference_code" => vec!["Identifier", "ReqId", "identifier", "reqid"],
        "description" => vec!["Statement", "Description", "statement", "description"],
        "status" => vec!["Status", "status"],
        "justification" => vec!["Rationale", "rationale"],
        _ => return None,
    };
    for k in keys {
        if let Some(v) = obj_attrs.get(k) {
            if !v.is_empty() {
                return Some(v.clone());
            }
        }
    }
    None
}
