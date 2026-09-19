// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! ReqIF 1.2 export: build XML from Marreq requirements.

use crate::models::Requirement;
use chrono::{SecondsFormat, Utc};
use std::collections::{HashMap, HashSet};

const REQIF_NS: &str = "http://www.omg.org/spec/ReqIF/20110401/reqif.xsd";

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn req_timestamp(req: &Requirement) -> String {
    format!("{}Z", req.update_date.format("%Y-%m-%dT%H:%M:%S"))
}

fn write_hierarchy(
    out: &mut String,
    requirement_id: i32,
    by_id: &HashMap<i32, &Requirement>,
    children: &HashMap<i32, Vec<i32>>,
    visited: &mut HashSet<i32>,
    indent: &str,
) {
    if !visited.insert(requirement_id) {
        return;
    }
    let Some(req) = by_id.get(&requirement_id) else {
        return;
    };
    out.push_str(&format!(
        "\n{indent}<SPEC-HIERARCHY IDENTIFIER=\"sh-{requirement_id}\" LAST-CHANGE=\"{}\">",
        req_timestamp(req)
    ));
    out.push_str(&format!(
        "\n{indent}  <OBJECT><SPEC-OBJECT-REF>so-{requirement_id}</SPEC-OBJECT-REF></OBJECT>"
    ));
    if let Some(child_ids) = children.get(&requirement_id) {
        out.push_str(&format!("\n{indent}  <CHILDREN>"));
        for child_id in child_ids {
            write_hierarchy(
                out,
                *child_id,
                by_id,
                children,
                visited,
                &format!("{indent}    "),
            );
        }
        out.push_str(&format!("\n{indent}  </CHILDREN>"));
    }
    out.push_str(&format!("\n{indent}</SPEC-HIERARCHY>"));
}

/// Build ReqIF 1.2 XML from project name, requirements, optional parent map (req_id -> parent_req_id),
/// and optional comments per requirement (req_id -> formatted remarks string).
pub fn to_reqif(
    project_name: &str,
    requirements: &[Requirement],
    parent_map: &HashMap<i32, i32>,
    comments_map: Option<&HashMap<i32, String>>,
) -> String {
    let generated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let mut out = String::new();
    out.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    out.push('\n');
    out.push_str(&format!(r#"<REQ-IF xmlns="{}">"#, REQIF_NS));
    out.push_str("\n  <THE-HEADER>");
    out.push_str("\n    <REQ-IF-HEADER IDENTIFIER=\"header-marreq\">");
    out.push_str("\n      <CREATION-TIME>");
    out.push_str(&generated_at);
    out.push_str("</CREATION-TIME>");
    out.push_str("\n      <REPOSITORY-ID>Marreq-");
    out.push_str(&escape_xml(project_name));
    out.push_str("</REPOSITORY-ID>");
    out.push_str("\n      <REQ-IF-TOOL-ID>Marreq</REQ-IF-TOOL-ID>");
    // ReqIF 1.2 uses the 20110401 schema whose REQ-IF-VERSION fixed value is 1.0.
    out.push_str("\n      <REQ-IF-VERSION>1.0</REQ-IF-VERSION>");
    out.push_str("\n      <SOURCE-TOOL-ID>Marreq</SOURCE-TOOL-ID>");
    out.push_str("\n      <TITLE>");
    out.push_str(&escape_xml(project_name));
    out.push_str("</TITLE>");
    out.push_str("\n    </REQ-IF-HEADER>");
    out.push_str("\n  </THE-HEADER>");
    out.push_str("\n  <CORE-CONTENT>");
    out.push_str("\n    <REQ-IF-CONTENT>");

    // Datatype definitions (STRING)
    out.push_str("\n      <DATATYPES>");
    out.push_str(&format!(
        "\n        <DATATYPE-DEFINITION-STRING IDENTIFIER=\"dt-string\" LAST-CHANGE=\"{generated_at}\" LONG-NAME=\"String\" MAX-LENGTH=\"2000000\"/>"
    ));
    out.push_str("\n      </DATATYPES>");

    // Spec types and their attribute definitions.
    out.push_str("\n      <SPEC-TYPES>");
    out.push_str(&format!(
        "\n        <SPEC-OBJECT-TYPE IDENTIFIER=\"sot-req\" LAST-CHANGE=\"{generated_at}\" LONG-NAME=\"Requirement\">"
    ));
    out.push_str("\n          <SPEC-ATTRIBUTES>");
    for (id, name) in [
        ("ad-identifier", "Identifier"),
        ("ad-title", "Title"),
        ("ad-statement", "Statement"),
        ("ad-rationale", "Rationale"),
        ("ad-remarks", "Remarks"),
    ] {
        out.push_str(&format!(
            "\n            <ATTRIBUTE-DEFINITION-STRING IDENTIFIER=\"{id}\" LAST-CHANGE=\"{generated_at}\" LONG-NAME=\"{name}\">"
        ));
        out.push_str(
            "\n              <TYPE><DATATYPE-DEFINITION-STRING-REF>dt-string</DATATYPE-DEFINITION-STRING-REF></TYPE>",
        );
        out.push_str("\n            </ATTRIBUTE-DEFINITION-STRING>");
    }
    out.push_str("\n          </SPEC-ATTRIBUTES>");
    out.push_str("\n        </SPEC-OBJECT-TYPE>");
    out.push_str(&format!(
        "\n        <SPEC-RELATION-TYPE IDENTIFIER=\"srt-parent\" LAST-CHANGE=\"{generated_at}\" LONG-NAME=\"DERIVES_FROM\"/>"
    ));
    out.push_str(&format!(
        "\n        <SPECIFICATION-TYPE IDENTIFIER=\"st-document\" LAST-CHANGE=\"{generated_at}\" LONG-NAME=\"Requirements document\"/>"
    ));
    out.push_str("\n      </SPEC-TYPES>");

    // Spec objects (one per requirement)
    out.push_str("\n      <SPEC-OBJECTS>");
    for req in requirements {
        let so_id = format!("so-{}", req.id);
        out.push_str("\n        <SPEC-OBJECT IDENTIFIER=\"");
        out.push_str(&escape_xml(&so_id));
        out.push_str("\" LAST-CHANGE=\"");
        out.push_str(&req_timestamp(req));
        out.push_str("\" LONG-NAME=\"");
        out.push_str(&escape_xml(&req.reference_code));
        out.push_str("\">");
        out.push_str("\n          <VALUES>");
        for (value, definition) in [
            (&req.reference_code, "ad-identifier"),
            (&req.title, "ad-title"),
            (&req.description, "ad-statement"),
        ] {
            out.push_str("\n            <ATTRIBUTE-VALUE-STRING THE-VALUE=\"");
            out.push_str(&escape_xml(value));
            out.push_str("\"><DEFINITION><ATTRIBUTE-DEFINITION-STRING-REF>");
            out.push_str(definition);
            out.push_str(
                "</ATTRIBUTE-DEFINITION-STRING-REF></DEFINITION></ATTRIBUTE-VALUE-STRING>",
            );
        }
        if let Some(ref j) = req.justification {
            if !j.is_empty() {
                out.push_str("\n            <ATTRIBUTE-VALUE-STRING THE-VALUE=\"");
                out.push_str(&escape_xml(j));
                out.push_str("\"><DEFINITION><ATTRIBUTE-DEFINITION-STRING-REF>ad-rationale</ATTRIBUTE-DEFINITION-STRING-REF></DEFINITION></ATTRIBUTE-VALUE-STRING>");
            }
        }
        if let Some(map) = comments_map {
            if let Some(remarks) = map.get(&req.id) {
                if !remarks.is_empty() {
                    out.push_str("\n            <ATTRIBUTE-VALUE-STRING THE-VALUE=\"");
                    out.push_str(&escape_xml(remarks));
                    out.push_str("\"><DEFINITION><ATTRIBUTE-DEFINITION-STRING-REF>ad-remarks</ATTRIBUTE-DEFINITION-STRING-REF></DEFINITION></ATTRIBUTE-VALUE-STRING>");
                }
            }
        }
        out.push_str("\n          </VALUES>");
        out.push_str(
            "\n          <TYPE><SPEC-OBJECT-TYPE-REF>sot-req</SPEC-OBJECT-TYPE-REF></TYPE>",
        );
        out.push_str("\n        </SPEC-OBJECT>");
    }
    out.push_str("\n      </SPEC-OBJECTS>");

    // Spec relations (parent-child)
    out.push_str("\n      <SPEC-RELATIONS>");
    for req in requirements {
        if let Some(parent_id) = parent_map.get(&req.id) {
            let sr_id = format!("sr-{}-{}", req.id, parent_id);
            out.push_str("\n        <SPEC-RELATION IDENTIFIER=\"");
            out.push_str(&escape_xml(&sr_id));
            out.push_str("\" LAST-CHANGE=\"");
            out.push_str(&req_timestamp(req));
            out.push_str("\" LONG-NAME=\"parent\">");
            out.push_str("\n          <SOURCE><SPEC-OBJECT-REF>so-");
            out.push_str(&req.id.to_string());
            out.push_str("</SPEC-OBJECT-REF></SOURCE>");
            out.push_str("\n          <TARGET><SPEC-OBJECT-REF>so-");
            out.push_str(&parent_id.to_string());
            out.push_str("</SPEC-OBJECT-REF></TARGET>");
            out.push_str(
                "\n          <TYPE><SPEC-RELATION-TYPE-REF>srt-parent</SPEC-RELATION-TYPE-REF></TYPE>",
            );
            out.push_str("\n        </SPEC-RELATION>");
        }
    }
    out.push_str("\n      </SPEC-RELATIONS>");

    // One specification preserving the parent-child tree.
    out.push_str("\n      <SPECIFICATIONS>");
    out.push_str(&format!(
        "\n        <SPECIFICATION IDENTIFIER=\"spec-main\" LAST-CHANGE=\"{generated_at}\" LONG-NAME=\""
    ));
    out.push_str(&escape_xml(project_name));
    out.push_str("\">");
    out.push_str(
        "\n          <TYPE><SPECIFICATION-TYPE-REF>st-document</SPECIFICATION-TYPE-REF></TYPE>",
    );
    out.push_str("\n          <CHILDREN>");

    let by_id: HashMap<i32, &Requirement> = requirements.iter().map(|req| (req.id, req)).collect();
    let mut children: HashMap<i32, Vec<i32>> = HashMap::new();
    let mut child_ids = HashSet::new();
    for (&child, &parent) in parent_map {
        if by_id.contains_key(&child) && by_id.contains_key(&parent) && child != parent {
            children.entry(parent).or_default().push(child);
            child_ids.insert(child);
        }
    }
    for ids in children.values_mut() {
        ids.sort_unstable();
    }
    let mut roots: Vec<i32> = requirements
        .iter()
        .map(|req| req.id)
        .filter(|id| !child_ids.contains(id))
        .collect();
    roots.sort_unstable();
    let mut visited = HashSet::new();
    for root in roots {
        write_hierarchy(
            &mut out,
            root,
            &by_id,
            &children,
            &mut visited,
            "            ",
        );
    }
    // Malformed/cyclic internal parent data must not omit requirements.
    for req in requirements {
        if !visited.contains(&req.id) {
            write_hierarchy(
                &mut out,
                req.id,
                &by_id,
                &children,
                &mut visited,
                "            ",
            );
        }
    }
    out.push_str("\n          </CHILDREN>");
    out.push_str("\n        </SPECIFICATION>");
    out.push_str("\n      </SPECIFICATIONS>");

    out.push_str("\n    </REQ-IF-CONTENT>");
    out.push_str("\n  </CORE-CONTENT>");
    out.push_str("\n</REQ-IF>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn requirement(id: i32, title: &str, reference: &str) -> Requirement {
        let timestamp = NaiveDate::from_ymd_opt(2026, 9, 19)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        Requirement {
            id,
            current_version_id: Some(id),
            same_as_current: None,
            title: title.into(),
            description: format!("{title} body"),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: reference.into(),
            category_id: 1,
            parent_id: None,
            creation_date: timestamp,
            update_date: timestamp,
            deadline_date: None,
            applicability_id: 1,
            justification: None,
            project_id: 1,
            approval_state: "draft".into(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        }
    }

    #[test]
    fn export_uses_reqif_schema_shapes_and_nested_hierarchy() {
        let requirements = vec![
            requirement(1, "Parent", "REQ-001"),
            requirement(2, "Child", "REQ-002"),
        ];
        let xml = to_reqif("Project", &requirements, &HashMap::from([(2, 1)]), None);

        assert!(xml.contains("<REQ-IF-HEADER IDENTIFIER=\"header-marreq\">"));
        assert!(xml.contains("<REQ-IF-VERSION>1.0</REQ-IF-VERSION>"));
        assert!(xml.contains(
            "<TYPE><DATATYPE-DEFINITION-STRING-REF>dt-string</DATATYPE-DEFINITION-STRING-REF></TYPE>"
        ));
        assert!(xml
            .contains("<TYPE><SPECIFICATION-TYPE-REF>st-document</SPECIFICATION-TYPE-REF></TYPE>"));
        assert!(xml.contains("<SOURCE><SPEC-OBJECT-REF>so-2</SPEC-OBJECT-REF></SOURCE>"));
        let parent_pos = xml.find("IDENTIFIER=\"sh-1\"").unwrap();
        let child_pos = xml.find("IDENTIFIER=\"sh-2\"").unwrap();
        assert!(child_pos > parent_pos);
        assert!(!xml.contains(" TYPE=\"sot-req\""));
        assert!(!xml.contains(" DEFINITION=\"ad-"));
    }
}
