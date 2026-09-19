// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Default ReqIF attribute long-name to Marreq field mapping.

use std::collections::HashMap;

/// Returns default mapping: ReqIF attribute long-name (case-insensitive match) -> Marreq field name.
pub fn default_attribute_mapping() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("Title".to_string(), "title".to_string());
    m.insert("ReqIF.Name".to_string(), "title".to_string());
    m.insert("ReqIF.ChapterName".to_string(), "title".to_string());
    m.insert("Identifier".to_string(), "reference_code".to_string());
    m.insert("ReqId".to_string(), "reference_code".to_string());
    m.insert("ReqIF.ForeignID".to_string(), "reference_code".to_string());
    m.insert("Statement".to_string(), "description".to_string());
    m.insert("Description".to_string(), "description".to_string());
    m.insert("ReqIF.Text".to_string(), "description".to_string());
    m.insert("Status".to_string(), "status".to_string());
    m.insert("Rationale".to_string(), "justification".to_string());
    m
}

fn lookup_exact(obj_attrs: &HashMap<String, String>, keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Some(v) = obj_attrs.get(*k) {
            if !v.is_empty() {
                return Some(v.clone());
            }
        }
    }
    None
}

fn lookup_ci(obj_attrs: &HashMap<String, String>, keys: &[&str]) -> Option<String> {
    for k in keys {
        let needle = k.to_ascii_lowercase();
        if let Some((_, v)) = obj_attrs
            .iter()
            .find(|(name, value)| !value.is_empty() && name.to_ascii_lowercase() == needle)
        {
            return Some(v.clone());
        }
    }
    None
}

/// Resolve attribute value by trying known long-names (Title, Identifier, Statement, etc.).
pub fn get_attr(obj_attrs: &HashMap<String, String>, field: &str) -> Option<String> {
    let keys: Vec<&str> = match field {
        "title" => vec!["Title", "title", "ReqIF.Name", "ReqIF.ChapterName"],
        "reference_code" => vec![
            "Identifier",
            "ReqId",
            "identifier",
            "reqid",
            "ReqIF.ForeignID",
            "ReqIF.ForeignId",
            "ForeignID",
        ],
        "description" => vec![
            "Statement",
            "Description",
            "statement",
            "description",
            "ReqIF.Text",
            "ReqIF.Description",
        ],
        "status" => vec!["Status", "status"],
        "justification" => vec!["Rationale", "rationale"],
        _ => return None,
    };
    lookup_exact(obj_attrs, &keys)
        .or_else(|| lookup_ci(obj_attrs, &keys))
        .or_else(|| {
            obj_attrs.iter().find_map(|(name, value)| {
                if value.is_empty() {
                    return None;
                }
                let normalized = name.to_ascii_uppercase();
                let matches = match field {
                    "title" => {
                        normalized.ends_with("-TITLE")
                            || normalized.ends_with("_TITLE")
                            || normalized.ends_with(".NAME")
                    }
                    "description" => {
                        normalized.ends_with("-TXT")
                            || normalized.ends_with("_TXT")
                            || normalized.ends_with(".TEXT")
                    }
                    "status" => normalized.ends_with("-STATUS") || normalized.ends_with("_STATUS"),
                    _ => false,
                };
                matches.then(|| value.clone())
            })
        })
}

/// True when a ReqIF attribute long-name maps onto a first-class Marreq field.
pub fn is_core_field(long_name: &str) -> bool {
    let mut one = HashMap::new();
    one.insert(long_name.to_string(), "x".to_string());
    get_attr(&one, "title").is_some()
        || get_attr(&one, "reference_code").is_some()
        || get_attr(&one, "description").is_some()
        || get_attr(&one, "status").is_some()
        || get_attr(&one, "justification").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn default_mapping_contains_expected_keys() {
        let m = default_attribute_mapping();
        assert_eq!(m.get("Title"), Some(&"title".to_string()));
        assert_eq!(m.get("Identifier"), Some(&"reference_code".to_string()));
        assert_eq!(m.get("Statement"), Some(&"description".to_string()));
        assert_eq!(m.get("Description"), Some(&"description".to_string()));
        assert_eq!(m.get("Status"), Some(&"status".to_string()));
        assert_eq!(m.get("Rationale"), Some(&"justification".to_string()));
        assert_eq!(m.get("ReqId"), Some(&"reference_code".to_string()));
        assert_eq!(m.get("ReqIF.Name"), Some(&"title".to_string()));
        assert_eq!(m.get("ReqIF.Text"), Some(&"description".to_string()));
    }

    #[test]
    fn get_attr_title() {
        let mut attrs = HashMap::new();
        attrs.insert("Title".to_string(), "My Title".to_string());
        assert_eq!(get_attr(&attrs, "title"), Some("My Title".to_string()));
        attrs.insert("title".to_string(), "lower".to_string());
        assert_eq!(get_attr(&attrs, "title"), Some("My Title".to_string()));
    }

    #[test]
    fn get_attr_reqif_name() {
        let mut attrs = HashMap::new();
        attrs.insert("ReqIF.Name".to_string(), "Section 1".to_string());
        assert_eq!(get_attr(&attrs, "title"), Some("Section 1".to_string()));
    }

    #[test]
    fn get_attr_reference_code() {
        let mut attrs = HashMap::new();
        attrs.insert("Identifier".to_string(), "REF-1".to_string());
        assert_eq!(
            get_attr(&attrs, "reference_code"),
            Some("REF-1".to_string())
        );
        attrs.insert("ReqId".to_string(), "REQ-2".to_string());
        assert_eq!(
            get_attr(&attrs, "reference_code"),
            Some("REF-1".to_string())
        );
    }

    #[test]
    fn get_attr_description() {
        let mut attrs = HashMap::new();
        attrs.insert("Statement".to_string(), "The requirement text.".to_string());
        assert_eq!(
            get_attr(&attrs, "description"),
            Some("The requirement text.".to_string())
        );
    }

    #[test]
    fn get_attr_justification() {
        let mut attrs = HashMap::new();
        attrs.insert("Rationale".to_string(), "Because.".to_string());
        assert_eq!(
            get_attr(&attrs, "justification"),
            Some("Because.".to_string())
        );
    }

    #[test]
    fn get_attr_empty_value_returns_none() {
        let mut attrs = HashMap::new();
        attrs.insert("Title".to_string(), "".to_string());
        assert_eq!(get_attr(&attrs, "title"), None);
    }

    #[test]
    fn get_attr_unknown_field_returns_none() {
        let mut attrs = HashMap::new();
        attrs.insert("Title".to_string(), "x".to_string());
        assert_eq!(get_attr(&attrs, "unknown_field"), None);
    }

    #[test]
    fn get_attr_empty_attrs_returns_none() {
        let attrs = HashMap::new();
        assert_eq!(get_attr(&attrs, "title"), None);
    }
}
