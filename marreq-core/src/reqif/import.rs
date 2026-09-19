// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! ReqIF 1.2 import: parse XML into intermediate model.
//!
//! Accepts both the nested-element encoding used by OMG ReqIF 1.2 (and by
//! DOORS, Polarion, RMF, Capella, EA, StrictDoc) and the flatter attribute
//! encoding historically emitted by Marreq itself.

use crate::reqif::mapping;
use crate::reqif::schema::{ParsedHierarchyEdge, ParsedSpecObject, ParsedSpecRelation};
use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::Reader;
use std::collections::HashMap;
use std::io::Cursor;

/// Parsed ReqIF document (SpecObjects, SpecRelations, and hierarchy edges).
#[derive(Debug, Default)]
pub struct ParsedDocument {
    pub objects: Vec<ParsedSpecObject>,
    pub relations: Vec<ParsedSpecRelation>,
    pub hierarchy_edges: Vec<ParsedHierarchyEdge>,
    pub specification_count: usize,
    pub object_type_count: usize,
    pub datatype_definition_count: usize,
    pub attribute_definition_count: usize,
    pub xhtml_value_count: usize,
    pub enumeration_value_count: usize,
    pub scalar_value_count: usize,
    pub attachment_count: usize,
    pub warnings: Vec<String>,
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
    pub created_link_count: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub imported_requirement_ids: Vec<i32>,
}

fn local_name(q: QName<'_>) -> String {
    String::from_utf8_lossy(q.local_name().as_ref()).into_owned()
}

fn is_attr_definition(name: &str) -> bool {
    name.starts_with("ATTRIBUTE-DEFINITION-")
}

fn is_attr_value(name: &str) -> bool {
    name.starts_with("ATTRIBUTE-VALUE-")
}

fn is_enum_value_def(name: &str) -> bool {
    name == "ENUM-VALUE"
}

struct HierFrame {
    object_id: Option<String>,
}

struct Parser {
    doc: ParsedDocument,
    open_elements: usize,
    attr_defs: HashMap<String, String>,
    enum_values: HashMap<String, String>,
    relation_type_names: HashMap<String, String>,
    current_object: Option<ParsedSpecObject>,
    current_relation: Option<ParsedSpecRelation>,
    current_def_id: Option<String>,
    current_value: Option<String>,
    current_attr_long_name: Option<String>,
    capturing_the_value: bool,
    the_value_depth: i32,
    /// Innermost structural context (SOURCE, TARGET, OBJECT, DEFINITION, TYPE, ENUM).
    container: Option<String>,
    hierarchy: Vec<HierFrame>,
    in_spec_object: bool,
    in_spec_relation: bool,
    in_specification: bool,
    in_spec_hierarchy: bool,
    saw_embedded: bool,
    saw_relation_attributes: bool,
    saw_specification_attributes: bool,
}

impl Parser {
    fn new() -> Self {
        Self {
            doc: ParsedDocument::default(),
            open_elements: 0,
            attr_defs: HashMap::new(),
            enum_values: HashMap::new(),
            relation_type_names: HashMap::new(),
            current_object: None,
            current_relation: None,
            current_def_id: None,
            current_value: None,
            current_attr_long_name: None,
            capturing_the_value: false,
            the_value_depth: 0,
            container: None,
            hierarchy: Vec::new(),
            in_spec_object: false,
            in_spec_relation: false,
            in_specification: false,
            in_spec_hierarchy: false,
            saw_embedded: false,
            saw_relation_attributes: false,
            saw_specification_attributes: false,
        }
    }

    fn start(&mut self, name: &str, e: &quick_xml::events::BytesStart<'_>) {
        match name {
            "SPEC-OBJECT" => {
                self.in_spec_object = true;
                self.current_object = Some(ParsedSpecObject {
                    id: attr(e, "IDENTIFIER").unwrap_or_default(),
                    type_ref: attr(e, "TYPE").unwrap_or_default(),
                    long_name: attr(e, "LONG-NAME"),
                    last_change: attr(e, "LAST-CHANGE"),
                    attributes: HashMap::new(),
                });
            }
            "SPEC-RELATION" => {
                self.in_spec_relation = true;
                self.current_relation = Some(ParsedSpecRelation {
                    id: attr(e, "IDENTIFIER").unwrap_or_default(),
                    type_ref: attr(e, "TYPE").unwrap_or_default(),
                    source: attr(e, "SOURCE").unwrap_or_default(),
                    target: attr(e, "TARGET").unwrap_or_default(),
                });
            }
            "SPEC-HIERARCHY" => {
                self.in_spec_hierarchy = true;
                self.hierarchy.push(HierFrame { object_id: None });
                if let Some(object) = attr(e, "OBJECT") {
                    self.set_hierarchy_object(object);
                }
            }
            "SPECIFICATION" => {
                self.doc.specification_count += 1;
                self.in_specification = true;
            }
            "SPEC-OBJECT-TYPE" => {
                self.doc.object_type_count += 1;
            }
            "SPEC-RELATION-TYPE" => {
                let id = attr(e, "IDENTIFIER").unwrap_or_default();
                let long_name = attr(e, "LONG-NAME").unwrap_or_default();
                if !id.is_empty() && !long_name.is_empty() {
                    self.relation_type_names.insert(id, long_name);
                }
            }
            n if n.starts_with("DATATYPE-DEFINITION-") => {
                self.doc.datatype_definition_count += 1;
            }
            n if is_attr_definition(n) => {
                self.doc.attribute_definition_count += 1;
                let id = attr(e, "IDENTIFIER").unwrap_or_default();
                let long_name = attr(e, "LONG-NAME").unwrap_or_default();
                if !id.is_empty() && !long_name.is_empty() {
                    self.attr_defs.insert(id, long_name);
                }
            }
            n if is_enum_value_def(n) => {
                let id = attr(e, "IDENTIFIER").unwrap_or_default();
                let long_name = attr(e, "LONG-NAME").unwrap_or_default();
                if !id.is_empty() {
                    self.enum_values.insert(
                        id.clone(),
                        if long_name.is_empty() { id } else { long_name },
                    );
                }
            }
            n if is_attr_value(n) => {
                match n {
                    "ATTRIBUTE-VALUE-XHTML" => self.doc.xhtml_value_count += 1,
                    "ATTRIBUTE-VALUE-ENUMERATION" => self.doc.enumeration_value_count += 1,
                    "ATTRIBUTE-VALUE-INTEGER"
                    | "ATTRIBUTE-VALUE-REAL"
                    | "ATTRIBUTE-VALUE-BOOLEAN"
                    | "ATTRIBUTE-VALUE-DATE" => self.doc.scalar_value_count += 1,
                    _ => {}
                }
                if self.in_spec_relation {
                    self.saw_relation_attributes = true;
                } else if self.in_specification && !self.in_spec_object {
                    self.saw_specification_attributes = true;
                }
                self.current_def_id = attr(e, "DEFINITION");
                self.current_value = attr(e, "THE-VALUE");
                self.current_attr_long_name = None;
            }
            "THE-VALUE" => {
                self.capturing_the_value = true;
                self.the_value_depth = 1;
            }
            "SOURCE" | "TARGET" | "OBJECT" | "DEFINITION" | "TYPE" => {
                self.container = Some(name.to_string());
            }
            "ENUM-VALUE-REF" => {
                self.container = Some("ENUM".to_string());
            }
            "FILE-NAME" => {
                self.saw_embedded = true;
                self.doc.attachment_count += 1;
            }
            "object" if attr(e, "data").is_some() => {
                self.saw_embedded = true;
                self.doc.attachment_count += 1;
            }
            _ => {}
        }
    }

    fn empty(&mut self, name: &str, e: &quick_xml::events::BytesStart<'_>) {
        self.start(name, e);
        self.end(name);
    }

    fn text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if self.capturing_the_value {
            match self.current_value.as_mut() {
                Some(existing) => {
                    if !existing.is_empty() {
                        existing.push(' ');
                    }
                    existing.push_str(text);
                }
                None => self.current_value = Some(text.to_string()),
            }
            return;
        }
        let Some(kind) = self.container.as_deref() else {
            return;
        };
        match kind {
            "SOURCE" => {
                if let Some(rel) = self.current_relation.as_mut() {
                    rel.source = text.to_string();
                }
            }
            "TARGET" => {
                if let Some(rel) = self.current_relation.as_mut() {
                    rel.target = text.to_string();
                }
            }
            "OBJECT" => {
                self.set_hierarchy_object(text.to_string());
            }
            "TYPE" => {
                if self.in_spec_object {
                    if let Some(obj) = self.current_object.as_mut() {
                        if obj.type_ref.is_empty() {
                            obj.type_ref = text.to_string();
                        }
                    }
                } else if self.in_spec_relation {
                    if let Some(rel) = self.current_relation.as_mut() {
                        if rel.type_ref.is_empty() {
                            rel.type_ref = text.to_string();
                        }
                    }
                }
            }
            "DEFINITION" => {
                self.current_def_id = Some(text.to_string());
            }
            "ENUM" => {
                let label = self
                    .enum_values
                    .get(text)
                    .cloned()
                    .unwrap_or_else(|| text.to_string());
                match self.current_value.as_mut() {
                    Some(existing) if !existing.is_empty() => {
                        existing.push_str(", ");
                        existing.push_str(&label);
                    }
                    _ => self.current_value = Some(label),
                }
            }
            _ => {}
        }
    }

    fn set_hierarchy_object(&mut self, object_id: String) {
        let parent_id = self
            .hierarchy
            .iter()
            .rev()
            .skip(1)
            .find_map(|frame| frame.object_id.clone());
        if let Some(frame) = self.hierarchy.last_mut() {
            frame.object_id = Some(object_id.clone());
        }
        if let Some(parent_id) = parent_id {
            if parent_id != object_id {
                self.doc.hierarchy_edges.push(ParsedHierarchyEdge {
                    child_id: object_id,
                    parent_id,
                });
            }
        }
    }

    fn finish_attr_value(&mut self) {
        let def_id = self.current_def_id.take();
        let value = self.current_value.take().unwrap_or_default();
        self.current_attr_long_name = None;
        self.capturing_the_value = false;
        self.the_value_depth = 0;
        let Some(obj) = self.current_object.as_mut() else {
            return;
        };
        let Some(def_id) = def_id else {
            return;
        };
        let long_name = self
            .attr_defs
            .get(&def_id)
            .cloned()
            .unwrap_or_else(|| def_id.clone());
        if !value.is_empty() {
            obj.attributes.insert(long_name, value);
        }
    }

    fn end(&mut self, name: &str) {
        if self.capturing_the_value {
            if name == "THE-VALUE" {
                self.capturing_the_value = false;
                self.the_value_depth = 0;
            }
            return;
        }
        match name {
            "SPEC-OBJECT" => {
                if let Some(obj) = self.current_object.take() {
                    self.doc.objects.push(obj);
                }
                self.in_spec_object = false;
            }
            "SPEC-RELATION" => {
                if let Some(mut rel) = self.current_relation.take() {
                    if let Some(long_name) = self.relation_type_names.get(&rel.type_ref) {
                        rel.type_ref = long_name.clone();
                    }
                    if !rel.source.is_empty() && !rel.target.is_empty() {
                        self.doc.relations.push(rel);
                    }
                }
                self.in_spec_relation = false;
            }
            "SPECIFICATION" => {
                self.in_specification = false;
            }
            "SPEC-HIERARCHY" => {
                self.hierarchy.pop();
                self.in_spec_hierarchy = !self.hierarchy.is_empty();
            }
            n if is_attr_value(n) => {
                self.finish_attr_value();
            }
            "SOURCE" | "TARGET" | "OBJECT" | "DEFINITION" | "TYPE" => {
                if self.container.as_deref() == Some(name) {
                    self.container = None;
                }
            }
            "ENUM-VALUE-REF" if self.container.as_deref() == Some("ENUM") => {
                self.container = None;
            }
            _ => {}
        }
    }

    fn finish(mut self) -> ParsedDocument {
        if self.saw_embedded {
            self.doc.warnings.push(
                "embedded/file content was present in the ReqIF document and was not imported"
                    .into(),
            );
        }
        if self.saw_relation_attributes {
            self.doc
                .warnings
                .push("SPEC-RELATION attributes were present and were not imported".into());
        }
        if self.saw_specification_attributes {
            self.doc
                .warnings
                .push("SPECIFICATION attributes were present and were not imported".into());
        }
        self.doc
    }
}

/// Parse ReqIF XML bytes into ParsedDocument.
pub fn parse_reqif(xml: &[u8]) -> Result<ParsedDocument, String> {
    let mut reader = Reader::from_reader(Cursor::new(xml));
    reader.config_mut().trim_text(true);

    let mut parser = Parser::new();
    let mut buf = Vec::new();
    let mut root_name: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                parser.open_elements += 1;
                let name = local_name(e.name());
                root_name.get_or_insert_with(|| name.clone());
                parser.start(&name, &e);
            }
            Ok(Event::Empty(e)) => {
                let name = local_name(e.name());
                root_name.get_or_insert_with(|| name.clone());
                parser.empty(&name, &e);
            }
            Ok(Event::End(e)) => {
                if parser.open_elements == 0 {
                    return Err("XML parse error: unmatched end tag".into());
                }
                parser.open_elements -= 1;
                let name = local_name(e.name());
                parser.end(&name);
            }
            Ok(Event::Text(e)) => {
                let text = e.xml10_content().unwrap_or_default();
                parser.text(text.trim());
            }
            Ok(Event::CData(e)) => {
                let text = String::from_utf8_lossy(e.as_ref());
                parser.text(text.trim());
            }
            Ok(Event::Eof) => {
                if parser.open_elements != 0 {
                    return Err("XML parse error: unclosed element".into());
                }
                break;
            }
            Err(e) => return Err(format!("XML parse error: {}", e)),
            _ => {}
        }
        buf.clear();
    }

    if root_name.as_deref() != Some("REQ-IF") {
        return Err("XML parse error: root element must be REQ-IF".into());
    }
    Ok(parser.finish())
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

/// Build Marreq field map from a parsed SpecObject using default attribute mapping.
pub fn object_to_fields(obj: &ParsedSpecObject) -> ReqifObjectFields {
    let mut title = mapping::get_attr(&obj.attributes, "title");
    if title.is_none() {
        title = obj.long_name.as_ref().filter(|s| !s.is_empty()).cloned();
    }
    let reference_code = mapping::get_attr(&obj.attributes, "reference_code");
    let description = mapping::get_attr(&obj.attributes, "description");
    let status = mapping::get_attr(&obj.attributes, "status");
    let justification = mapping::get_attr(&obj.attributes, "justification");
    (title, reference_code, description, status, justification)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn obj(id: &str, attrs: HashMap<String, String>) -> ParsedSpecObject {
        ParsedSpecObject {
            id: id.into(),
            type_ref: "type1".into(),
            long_name: None,
            last_change: None,
            attributes: attrs,
        }
    }

    #[test]
    fn parsed_document_default() {
        let doc = ParsedDocument::default();
        assert!(doc.objects.is_empty());
        assert!(doc.relations.is_empty());
        assert!(doc.hierarchy_edges.is_empty());
    }

    #[test]
    fn object_to_fields_empty_attrs() {
        let (t, r, d, s, j) = object_to_fields(&obj("id1", HashMap::new()));
        assert_eq!(t, None);
        assert_eq!(r, None);
        assert_eq!(d, None);
        assert_eq!(s, None);
        assert_eq!(j, None);
    }

    #[test]
    fn object_to_fields_title_only() {
        let mut attrs = HashMap::new();
        attrs.insert("Title".to_string(), "Req 1".to_string());
        let (t, r, d, s, j) = object_to_fields(&obj("id1", attrs));
        assert_eq!(t, Some("Req 1".into()));
        assert_eq!(r, None);
        assert_eq!(d, None);
        assert_eq!(s, None);
        assert_eq!(j, None);
    }

    #[test]
    fn object_to_fields_multiple() {
        let mut attrs = HashMap::new();
        attrs.insert("Title".to_string(), "T".to_string());
        attrs.insert("Identifier".to_string(), "REF-1".to_string());
        attrs.insert("Statement".to_string(), "Desc".to_string());
        let (t, r, d, s, j) = object_to_fields(&obj("id1", attrs));
        assert_eq!(t, Some("T".into()));
        assert_eq!(r, Some("REF-1".into()));
        assert_eq!(d, Some("Desc".into()));
        assert_eq!(s, None);
        assert_eq!(j, None);
    }

    #[test]
    fn parse_reqif_minimal_empty() {
        let xml = r#"<?xml version="1.0"?><REQ-IF xmlns="http://www.omg.org/ReqIF"></REQ-IF>"#;
        let result = parse_reqif(xml.as_bytes());
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert!(doc.objects.is_empty());
        assert!(doc.relations.is_empty());
    }

    #[test]
    fn parse_reqif_invalid_xml_returns_err() {
        let xml = b"<open><no close";
        let result = parse_reqif(xml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_lowercase().contains("parse"));
    }

    #[test]
    fn parse_reqif_one_spec_object() {
        let xml = r#"<?xml version="1.0"?>
        <REQ-IF>
          <CORE-CONTENT>
            <REQ-IF-CONTENT>
              <SPEC-TYPES><SPEC-OBJECT-TYPE IDENTIFIER="type1">
                <SPEC-ATTRIBUTES><ATTRIBUTE-DEFINITION-STRING IDENTIFIER="ad1" LONG-NAME="Title"/></SPEC-ATTRIBUTES>
              </SPEC-OBJECT-TYPE></SPEC-TYPES>
              <SPEC-OBJECTS>
                <SPEC-OBJECT IDENTIFIER="obj1" TYPE="type1">
                  <VALUES><ATTRIBUTE-VALUE-STRING DEFINITION="ad1" THE-VALUE="My Title"/></VALUES>
                </SPEC-OBJECT>
              </SPEC-OBJECTS>
            </REQ-IF-CONTENT>
          </CORE-CONTENT>
        </REQ-IF>"#;
        let result = parse_reqif(xml.as_bytes());
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.objects.len(), 1);
        assert_eq!(doc.objects[0].id, "obj1");
        assert_eq!(doc.objects[0].type_ref, "type1");
        assert_eq!(
            doc.objects[0].attributes.get("Title"),
            Some(&"My Title".into())
        );
    }

    #[test]
    fn parse_reqif_nested_definition_and_relation() {
        let xml = r#"<?xml version="1.0"?>
        <reqif:REQ-IF xmlns:reqif="http://www.omg.org/spec/ReqIF/20110401/reqif.xsd">
          <reqif:CORE-CONTENT>
            <reqif:REQ-IF-CONTENT>
              <reqif:SPEC-TYPES>
                <reqif:SPEC-OBJECT-TYPE IDENTIFIER="type1">
                  <reqif:SPEC-ATTRIBUTES>
                    <reqif:ATTRIBUTE-DEFINITION-STRING IDENTIFIER="ad1" LONG-NAME="ReqIF.Name"/>
                    <reqif:ATTRIBUTE-DEFINITION-XHTML IDENTIFIER="ad2" LONG-NAME="ReqIF.Text"/>
                  </reqif:SPEC-ATTRIBUTES>
                </reqif:SPEC-OBJECT-TYPE>
              </reqif:SPEC-TYPES>
              <reqif:SPEC-OBJECTS>
                <reqif:SPEC-OBJECT IDENTIFIER="A" LONG-NAME="Heading A">
                  <reqif:TYPE><reqif:SPEC-OBJECT-TYPE-REF>type1</reqif:SPEC-OBJECT-TYPE-REF></reqif:TYPE>
                  <reqif:VALUES>
                    <reqif:ATTRIBUTE-VALUE-STRING THE-VALUE="Alpha">
                      <reqif:DEFINITION>
                        <reqif:ATTRIBUTE-DEFINITION-STRING-REF>ad1</reqif:ATTRIBUTE-DEFINITION-STRING-REF>
                      </reqif:DEFINITION>
                    </reqif:ATTRIBUTE-VALUE-STRING>
                    <reqif:ATTRIBUTE-VALUE-XHTML>
                      <reqif:DEFINITION>
                        <reqif:ATTRIBUTE-DEFINITION-XHTML-REF>ad2</reqif:ATTRIBUTE-DEFINITION-XHTML-REF>
                      </reqif:DEFINITION>
                      <reqif:THE-VALUE><xhtml:p xmlns:xhtml="http://www.w3.org/1999/xhtml">Body of A</xhtml:p></reqif:THE-VALUE>
                    </reqif:ATTRIBUTE-VALUE-XHTML>
                  </reqif:VALUES>
                </reqif:SPEC-OBJECT>
                <reqif:SPEC-OBJECT IDENTIFIER="B">
                  <reqif:VALUES>
                    <reqif:ATTRIBUTE-VALUE-STRING THE-VALUE="Beta">
                      <reqif:DEFINITION>
                        <reqif:ATTRIBUTE-DEFINITION-STRING-REF>ad1</reqif:ATTRIBUTE-DEFINITION-STRING-REF>
                      </reqif:DEFINITION>
                    </reqif:ATTRIBUTE-VALUE-STRING>
                  </reqif:VALUES>
                </reqif:SPEC-OBJECT>
              </reqif:SPEC-OBJECTS>
              <reqif:SPEC-RELATIONS>
                <reqif:SPEC-RELATION IDENTIFIER="rel1">
                  <reqif:SOURCE><reqif:SPEC-OBJECT-REF>B</reqif:SPEC-OBJECT-REF></reqif:SOURCE>
                  <reqif:TARGET><reqif:SPEC-OBJECT-REF>A</reqif:SPEC-OBJECT-REF></reqif:TARGET>
                </reqif:SPEC-RELATION>
              </reqif:SPEC-RELATIONS>
              <reqif:SPECIFICATIONS>
                <reqif:SPECIFICATION IDENTIFIER="spec1">
                  <reqif:CHILDREN>
                    <reqif:SPEC-HIERARCHY IDENTIFIER="h1">
                      <reqif:OBJECT><reqif:SPEC-OBJECT-REF>A</reqif:SPEC-OBJECT-REF></reqif:OBJECT>
                      <reqif:CHILDREN>
                        <reqif:SPEC-HIERARCHY IDENTIFIER="h2">
                          <reqif:OBJECT><reqif:SPEC-OBJECT-REF>B</reqif:SPEC-OBJECT-REF></reqif:OBJECT>
                        </reqif:SPEC-HIERARCHY>
                      </reqif:CHILDREN>
                    </reqif:SPEC-HIERARCHY>
                  </reqif:CHILDREN>
                </reqif:SPECIFICATION>
              </reqif:SPECIFICATIONS>
            </reqif:REQ-IF-CONTENT>
          </reqif:CORE-CONTENT>
        </reqif:REQ-IF>"#;
        let doc = parse_reqif(xml.as_bytes()).unwrap();
        assert_eq!(doc.objects.len(), 2);
        assert_eq!(
            doc.objects[0].attributes.get("ReqIF.Name"),
            Some(&"Alpha".into())
        );
        assert_eq!(
            doc.objects[0]
                .attributes
                .get("ReqIF.Text")
                .map(|s| s.as_str()),
            Some("Body of A")
        );
        assert_eq!(doc.relations.len(), 1);
        assert_eq!(doc.relations[0].source, "B");
        assert_eq!(doc.relations[0].target, "A");
        assert_eq!(doc.hierarchy_edges.len(), 1);
        assert_eq!(doc.hierarchy_edges[0].child_id, "B");
        assert_eq!(doc.hierarchy_edges[0].parent_id, "A");
        assert_eq!(doc.specification_count, 1);
    }

    #[test]
    fn parse_reqif_unknown_elements_are_ignored() {
        let xml = r#"<?xml version="1.0"?><REQ-IF><CORE-CONTENT><VENDOR-EXT foo="1"/><REQ-IF-CONTENT>
            <SPEC-OBJECTS><SPEC-OBJECT IDENTIFIER="x"/></SPEC-OBJECTS>
        </REQ-IF-CONTENT></CORE-CONTENT></REQ-IF>"#;
        let doc = parse_reqif(xml.as_bytes()).unwrap();
        assert_eq!(doc.objects.len(), 1);
        assert_eq!(doc.objects[0].id, "x");
    }
}
