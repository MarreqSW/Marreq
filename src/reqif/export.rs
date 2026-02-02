//! ReqIF 1.2 export: build XML from ReqMan requirements.

use crate::models::Requirement;
use std::collections::HashMap;

const REQIF_NS: &str = "http://www.omg.org/spec/ReqIF/20110401/reqif.xsd";

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Build ReqIF 1.2 XML from project name, requirements, and optional parent map (req_id -> parent_req_id).
pub fn to_reqif(
    project_name: &str,
    requirements: &[Requirement],
    parent_map: &HashMap<i32, i32>,
) -> String {
    let mut out = String::new();
    out.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    out.push_str("\n");
    out.push_str(&format!(
        r#"<REQ-IF xmlns="{}">"#,
        REQIF_NS
    ));
    out.push_str("\n  <THE-HEADER>");
    out.push_str("\n    <REPOSITORY-ID>ReqMan-");
    out.push_str(&escape_xml(project_name));
    out.push_str("</REPOSITORY-ID>");
    out.push_str("\n    <REQ-IF-TOOL-ID>ReqMan</REQ-IF-TOOL-ID>");
    out.push_str("\n    <REQ-IF-VERSION>1.2</REQ-IF-VERSION>");
    out.push_str("\n    <SOURCE-TOOL-ID>ReqMan</SOURCE-TOOL-ID>");
    out.push_str("\n    <TITLE>");
    out.push_str(&escape_xml(project_name));
    out.push_str("</TITLE>");
    out.push_str("\n  </THE-HEADER>");
    out.push_str("\n  <CORE-CONTENT>");
    out.push_str("\n    <REQ-IF-CONTENT>");

    // Datatype definitions (STRING)
    out.push_str("\n      <DATATYPES>");
    out.push_str("\n        <DATATYPE-DEFINITION-STRING IDENTIFIER=\"dt-string\" LONG-NAME=\"String\"/>");
    out.push_str("\n      </DATATYPES>");

    // Spec object type "Requirement" with attributes
    out.push_str("\n      <SPEC-TYPES>");
    out.push_str("\n        <SPEC-OBJECT-TYPE IDENTIFIER=\"sot-req\" LONG-NAME=\"Requirement\">");
    out.push_str("\n          <SPEC-ATTRIBUTES>");
    out.push_str("\n            <ATTRIBUTE-DEFINITION-STRING IDENTIFIER=\"ad-identifier\" LONG-NAME=\"Identifier\" TYPE=\"dt-string\"/>");
    out.push_str("\n            <ATTRIBUTE-DEFINITION-STRING IDENTIFIER=\"ad-title\" LONG-NAME=\"Title\" TYPE=\"dt-string\"/>");
    out.push_str("\n            <ATTRIBUTE-DEFINITION-STRING IDENTIFIER=\"ad-statement\" LONG-NAME=\"Statement\" TYPE=\"dt-string\"/>");
    out.push_str("\n            <ATTRIBUTE-DEFINITION-STRING IDENTIFIER=\"ad-rationale\" LONG-NAME=\"Rationale\" TYPE=\"dt-string\"/>");
    out.push_str("\n          </SPEC-ATTRIBUTES>");
    out.push_str("\n        </SPEC-OBJECT-TYPE>");
    out.push_str("\n      </SPEC-TYPES>");

    // Spec objects (one per requirement)
    out.push_str("\n      <SPEC-OBJECTS>");
    for req in requirements {
        let so_id = format!("so-{}", req.id);
        out.push_str("\n        <SPEC-OBJECT IDENTIFIER=\"");
        out.push_str(&escape_xml(&so_id));
        out.push_str("\" LONG-NAME=\"");
        out.push_str(&escape_xml(&req.reference_code));
        out.push_str("\" TYPE=\"sot-req\">");
        out.push_str("\n          <VALUES>");
        out.push_str("\n            <ATTRIBUTE-VALUE-STRING THE-VALUE=\"");
        out.push_str(&escape_xml(&req.reference_code));
        out.push_str("\" DEFINITION=\"ad-identifier\"/>");
        out.push_str("\n            <ATTRIBUTE-VALUE-STRING THE-VALUE=\"");
        out.push_str(&escape_xml(&req.title));
        out.push_str("\" DEFINITION=\"ad-title\"/>");
        out.push_str("\n            <ATTRIBUTE-VALUE-STRING THE-VALUE=\"");
        out.push_str(&escape_xml(&req.description));
        out.push_str("\" DEFINITION=\"ad-statement\"/>");
        if let Some(ref j) = req.justification {
            if !j.is_empty() {
                out.push_str("\n            <ATTRIBUTE-VALUE-STRING THE-VALUE=\"");
                out.push_str(&escape_xml(j));
                out.push_str("\" DEFINITION=\"ad-rationale\"/>");
            }
        }
        out.push_str("\n          </VALUES>");
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
            out.push_str("\" LONG-NAME=\"parent\" SOURCE=\"so-");
            out.push_str(&req.id.to_string());
            out.push_str("\" TARGET=\"so-");
            out.push_str(&parent_id.to_string());
            out.push_str("\"/>");
        }
    }
    out.push_str("\n      </SPEC-RELATIONS>");

    // One flat specification containing all (no hierarchy in MVP)
    out.push_str("\n      <SPECIFICATIONS>");
    out.push_str("\n        <SPECIFICATION IDENTIFIER=\"spec-main\" LONG-NAME=\"");
    out.push_str(&escape_xml(project_name));
    out.push_str("\">");
    out.push_str("\n          <TYPE><SPEC-OBJECT-TYPE-REF>sot-req</SPEC-OBJECT-TYPE-REF></TYPE>");
    out.push_str("\n          <CHILDREN>");
    for req in requirements {
        out.push_str("\n            <SPEC-HIERARCHY IDENTIFIER=\"sh-");
        out.push_str(&req.id.to_string());
        out.push_str("\" OBJECT=\"so-");
        out.push_str(&req.id.to_string());
        out.push_str("\"/>");
    }
    out.push_str("\n          </CHILDREN>");
    out.push_str("\n        </SPECIFICATION>");
    out.push_str("\n      </SPECIFICATIONS>");

    out.push_str("\n    </REQ-IF-CONTENT>");
    out.push_str("\n  </CORE-CONTENT>");
    out.push_str("\n</REQ-IF>");
    out
}
