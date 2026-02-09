//! ReqIF 1.2 import: parse XML into intermediate model.

use crate::reqif::mapping;
use crate::reqif::schema::{ParsedSpecObject, ParsedSpecRelation};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::io::Cursor;

/// Parsed ReqIF document (SpecObjects and SpecRelations).
#[derive(Debug, Default)]
pub struct ParsedDocument {
    pub objects: Vec<ParsedSpecObject>,
    pub relations: Vec<ParsedSpecRelation>,
}

/// Import configuration (target project, default status, etc.).
#[derive(Debug, Clone)]
pub struct ImportConfig {
    pub project_id: i32,
    pub default_status_id: i32,
    pub default_category_id: i32,
    pub default_applicability_id: i32,
    pub default_verification_method_id: i32,
    pub author_id: i32,
    pub reviewer_id: i32,
}

/// Result of ReqIF import (aligned with Excel import).
#[derive(Debug)]
pub struct ImportResult {
    pub success: bool,
    pub message: String,
    pub imported_count: usize,
    pub errors: Vec<String>,
    pub imported_requirement_ids: Vec<i32>,
}

/// Parse ReqIF XML bytes into ParsedDocument.
pub fn parse_reqif(xml: &[u8]) -> Result<ParsedDocument, String> {
    let mut reader = Reader::from_reader(Cursor::new(xml));
    reader.config_mut().trim_text(true);

    let mut doc = ParsedDocument::default();
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();

    // Attribute definition ID -> LONG-NAME (for mapping THE-VALUE to attribute name)
    let mut attr_defs: HashMap<String, String> = HashMap::new();
    let mut current_object: Option<ParsedSpecObject> = None;
    let mut current_attr_value_def: Option<String> = None;
    let mut current_attr_value: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                stack.push(name.clone());

                match name.as_str() {
                    "SPEC-OBJECT" => {
                        let id = attr(&e, "IDENTIFIER").unwrap_or_default();
                        let type_ref = attr(&e, "TYPE").unwrap_or_default();
                        current_object = Some(ParsedSpecObject {
                            id,
                            type_ref,
                            attributes: HashMap::new(),
                        });
                    }
                    "ATTRIBUTE-DEFINITION-STRING" | "ATTRIBUTE-DEFINITION-XHTML" => {
                        let id = attr(&e, "IDENTIFIER").unwrap_or_default();
                        let long_name = attr(&e, "LONG-NAME").unwrap_or_default();
                        if !id.is_empty() && !long_name.is_empty() {
                            attr_defs.insert(id.clone(), long_name);
                        }
                    }
                    "ATTRIBUTE-VALUE-STRING" | "ATTRIBUTE-VALUE-XHTML" => {
                        current_attr_value_def = attr(&e, "DEFINITION");
                        current_attr_value = attr(&e, "THE-VALUE");
                    }
                    "SPEC-RELATION" => {
                        let id = attr(&e, "IDENTIFIER").unwrap_or_default();
                        let source = attr(&e, "SOURCE").unwrap_or_default();
                        let target = attr(&e, "TARGET").unwrap_or_default();
                        let type_ref = attr(&e, "TYPE").unwrap_or_default();
                        if !source.is_empty() && !target.is_empty() {
                            doc.relations.push(ParsedSpecRelation {
                                id,
                                type_ref,
                                source,
                                target,
                            });
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if stack.last().map(|s| s.as_str()) == Some(name.as_str()) {
                    stack.pop();
                }

                match name.as_str() {
                    "SPEC-OBJECT" => {
                        if let Some(obj) = current_object.take() {
                            doc.objects.push(obj);
                        }
                    }
                    "ATTRIBUTE-VALUE-STRING" | "ATTRIBUTE-VALUE-XHTML" => {
                        if let (Some(obj), Some(def_id), value) = (
                            current_object.as_mut(),
                            current_attr_value_def.take(),
                            current_attr_value.take().or_else(|| Some(String::new())),
                        ) {
                            let long_name = attr_defs
                                .get(&def_id)
                                .cloned()
                                .unwrap_or_else(|| def_id.clone());
                            if let Some(v) = value {
                                obj.attributes.insert(long_name, v);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.xml10_content().unwrap_or_default().trim().to_string();
                if let Some(ref mut obj) = current_object {
                    if let Some(ref def_id) = current_attr_value_def {
                        let long_name = attr_defs
                            .get(def_id)
                            .cloned()
                            .unwrap_or_else(|| def_id.clone());
                        if !text.is_empty() {
                            obj.attributes
                                .entry(long_name)
                                .and_modify(|v| v.push_str(&format!(" {}", text)))
                                .or_insert_with(|| text.clone());
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    Ok(doc)
}

fn attr(e: &quick_xml::events::BytesStart<'_>, key: &str) -> Option<String> {
    let key_bytes = key.as_bytes();
    for a in e.attributes().flatten() {
        if a.key.as_ref() == key_bytes || a.key.local_name().as_ref() == key_bytes {
            return String::from_utf8(a.value.into_owned()).ok();
        }
    }
    None
}

/// Tuple of (title, reference_code, description, status, justification) from a parsed SpecObject.
pub type ReqifObjectFields = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// Build ReqMan field map from a parsed SpecObject using default attribute mapping.
pub fn object_to_fields(obj: &ParsedSpecObject) -> ReqifObjectFields {
    let title = mapping::get_attr(&obj.attributes, "title");
    let reference_code = mapping::get_attr(&obj.attributes, "reference_code");
    let description = mapping::get_attr(&obj.attributes, "description");
    let status = mapping::get_attr(&obj.attributes, "status");
    let justification = mapping::get_attr(&obj.attributes, "justification");
    (title, reference_code, description, status, justification)
}
