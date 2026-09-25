// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Vendor-fixture and robustness tests for ReqIF import.
//! Fixtures live in `tests/reqif/fixtures` (repo root) and are not modified.

use super::import::{object_to_fields, parse_reqif, ImportConfig};
use crate::app::{AppState, DieselCachedRepo};
use crate::models::{
    Applicability, Category, Project, RequirementStatus, User, VerificationMethod,
};
use crate::repository::diesel_repo_mock::DieselRepoMock;
use crate::repository::CacheRepository;
use crate::services::{ReqIFService, RequirementService};
use crate::status_enums::ProjectStatus;
use chrono::{NaiveDate, NaiveDateTime};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/reqif/fixtures")
}

fn load_fixture(rel: &str) -> Vec<u8> {
    let path = fixtures_dir().join(rel);
    fs::read(&path).unwrap_or_else(|e| panic!("could not read fixture {}: {e}", path.display()))
}

fn epoch() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(1970, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

fn blank_project() -> (AppState<DieselCachedRepo>, User, ImportConfig) {
    let mut repo = DieselRepoMock::default();
    let mut user = DieselRepoMock::make_user(1, "importer", "hash");
    user.is_admin = true;
    repo.users.insert(1, user.clone());
    repo.projects.insert(
        1,
        Project {
            id: 1,
            name: "ReqIF audit".into(),
            description: None,
            creation_date: Some(epoch()),
            update_date: Some(epoch()),
            status: ProjectStatus::Active,
            owner_id: Some(1),
            slug: "reqif-audit".into(),
            group_id: None,
        },
    );
    repo.requirement_statuses.insert(
        1,
        RequirementStatus {
            id: 1,
            title: "Draft".into(),
            description: String::new(),
            tag: "D".into(),
            project_id: 1,
            is_system: true,
            tag_color: None,
        },
    );
    repo.categories.insert(
        1,
        Category {
            id: 1,
            title: "Default".into(),
            description: String::new(),
            tag: "DEF".into(),
            project_id: 1,
        },
    );
    repo.applicability.insert(
        1,
        Applicability {
            id: 1,
            title: "All".into(),
            description: String::new(),
            tag: "ALL".into(),
            project_id: 1,
        },
    );
    repo.verification_methods.insert(
        1,
        VerificationMethod {
            id: 1,
            title: "Test".into(),
            description: String::new(),
            tag: "T".into(),
            project_id: 1,
        },
    );
    let state = AppState {
        repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
    };
    let config = ImportConfig {
        project_id: 1,
        default_status_id: 1,
        default_category_id: 1,
        default_applicability_id: 1,
        default_verification_method_id: 1,
        author_id: 1,
        reviewer_id: 1,
    };
    (state, user, config)
}

fn expected_link_count(doc: &crate::reqif::import::ParsedDocument) -> usize {
    let mut pairs = HashSet::new();
    for edge in &doc.hierarchy_edges {
        if edge.child_id != edge.parent_id {
            pairs.insert((edge.child_id.clone(), edge.parent_id.clone()));
        }
    }
    for rel in &doc.relations {
        if !rel.source.is_empty() && !rel.target.is_empty() && rel.source != rel.target {
            pairs.insert((rel.source.clone(), rel.target.clone()));
        }
    }
    pairs.len()
}

fn import_fixture(
    rel: &str,
) -> (
    crate::reqif::import::ParsedDocument,
    crate::reqif::import::ImportResult,
) {
    let xml = load_fixture(rel);
    let doc = parse_reqif(&xml).unwrap_or_else(|e| panic!("{rel}: parse failed: {e}"));
    let (state, user, config) = blank_project();
    let service = ReqIFService::new(&state);
    let result = service
        .import_into_project(&xml, &config, &user)
        .unwrap_or_else(|e| panic!("{rel}: import failed: {e}"));
    (doc, result)
}

#[test]
fn test_reqif_capella_import() {
    let (doc, result) = import_fixture("capella/eclipse_capella_Sample.xml");
    assert_eq!(doc.objects.len(), 1);
    assert_eq!(doc.specification_count, 1);
    assert_eq!(result.imported_count, 1);
    assert!(result.success, "{:?}", result.errors);
    let (title, _, desc, _, _) = object_to_fields(&doc.objects[0]);
    assert_eq!(title.as_deref(), Some("Requirement-1"));
    assert_eq!(
        desc.as_deref(),
        Some("Requirement-1"),
        "XHTML text should be flattened into description"
    );
}

#[test]
fn test_reqif_doors_import() {
    let (doc, result) = import_fixture("doors/capella_Sample.reqif");
    assert_eq!(
        doc.objects.len(),
        1,
        "IBM DOORS Sample.reqif from Capella VP"
    );
    assert_eq!(result.imported_count, 1);
    assert!(result.success, "{:?}", result.errors);

    for fixture in [
        "doors/strictdoc_doors_date.reqif",
        "doors/strictdoc_doors_user.reqif",
    ] {
        let (vendor_doc, vendor_result) = import_fixture(fixture);
        assert_eq!(
            vendor_result.imported_count,
            vendor_doc.objects.len(),
            "{fixture}: {:?}",
            vendor_result.errors
        );
    }
}

#[test]
fn test_reqif_polarion_import() {
    let (doc, result) = import_fixture("polarion/strictdoc_04_sample1_polarion.reqif");
    assert_eq!(doc.objects.len(), 2);
    assert_eq!(doc.specification_count, 1);
    assert_eq!(result.imported_count, 2);
    assert_eq!(result.created_link_count, expected_link_count(&doc));
    assert!(result.success, "{:?}", result.errors);
    let titles: Vec<_> = doc.objects.iter().map(|o| object_to_fields(o).0).collect();
    assert!(titles.iter().any(|t| t.as_deref() == Some("Section 1")));

    let (doc_x, result_x) = import_fixture("polarion/polarion_export.xml");
    assert_eq!(doc_x.objects.len(), 1);
    assert_eq!(result_x.imported_count, 1);

    let (large_doc, large_result) = import_fixture("polarion/strictdoc_polarion_anonymized.reqif");
    assert_eq!(large_doc.objects.len(), 101);
    assert_eq!(large_result.imported_count, 101);
    assert_eq!(
        large_result.created_link_count,
        large_doc.hierarchy_edges.len()
    );
    assert!(large_result.success, "{:?}", large_result.errors);

    let (trace_doc, trace_result) =
        import_fixture("polarion/antcc_MAG8000-LTE-FeatureSpecReqBL4.reqif");
    assert_eq!(trace_doc.objects.len(), 102);
    assert_eq!(trace_doc.relations.len(), 26);
    assert_eq!(trace_doc.hierarchy_edges.len(), 100);
    assert_eq!(trace_result.imported_count, 102);
    assert_eq!(trace_result.created_link_count, 100);
    assert_eq!(
        trace_result
            .warnings
            .iter()
            .filter(|warning| warning.contains("SPEC-RELATION"))
            .count(),
        26,
        "every unrepresentable Polarion relation must be reported"
    );
}

#[test]
fn test_reqif_eclipse_rmf_import() {
    let (doc, result) = import_fixture("eclipse-rmf/eclipse_rmf_sample.reqif");
    assert_eq!(doc.objects.len(), 2);
    assert_eq!(result.imported_count, 2);
    assert!(result.success, "{:?}", result.errors);

    let (doc_rel, result_rel) = import_fixture("eclipse-rmf/eclipse_rmf_specRelationTest.reqif");
    assert_eq!(doc_rel.objects.len(), 2);
    assert_eq!(doc_rel.relations.len(), 1);
    assert_eq!(result_rel.imported_count, 2);
    assert_eq!(result_rel.created_link_count, expected_link_count(&doc_rel));
}

#[test]
fn test_reqif_enterprise_architect_import() {
    let (doc, result) = import_fixture("enterprise-architect/ea_example.reqif.xml");
    assert_eq!(doc.objects.len(), 3);
    assert_eq!(doc.relations.len(), 1);
    assert_eq!(doc.object_type_count, 8);
    assert_eq!(result.imported_count, 0);
    assert_eq!(result.created_link_count, 0);
    assert!(
        !result.success,
        "the fixture contains dangling FUNC-REQ-1/FUNC-REQ-2 references"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|error| error.contains("missing child SpecObject")),
        "dangling hierarchy references must be reported: {:?}",
        result.errors
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|error| error.contains("SPEC-RELATION")),
        "dangling relation references must be reported: {:?}",
        result.warnings
    );
    let titles: Vec<String> = doc
        .objects
        .iter()
        .filter_map(|o| object_to_fields(o).0)
        .collect();
    assert!(titles.iter().any(|t| t.contains("earths surface")));

    let (direct_doc, direct_result) = import_fixture("enterprise-architect/strictdoc_ea8.reqif");
    assert_eq!(direct_doc.objects.len(), 3);
    assert_eq!(direct_result.imported_count, 0);
    assert!(
        !direct_result.success,
        "StrictDoc's EA 8 fixture has dangling FUNC-REQ references"
    );
}

#[test]
fn test_reqif_strictdoc_import() {
    let xml = load_fixture("strictdoc/strictdoc_01_minimal_reqif_sample.reqif");
    let doc = parse_reqif(&xml).unwrap();
    assert!(doc.objects.is_empty(), "minimal header-only sample");

    let (doc, result) = import_fixture("strictdoc/strictdoc_04_sample2_sdoc.reqif");
    assert_eq!(doc.objects.len(), 18);
    assert_eq!(doc.specification_count, 1);
    assert!(!doc.hierarchy_edges.is_empty());
    assert_eq!(result.imported_count, 18);
    assert_eq!(result.created_link_count, expected_link_count(&doc));
    assert!(result.success, "{:?}", result.errors);
}

#[test]
fn test_reqif_studio_import() {
    let (doc, result) = import_fixture("reqif-studio/strictdoc_reqif_studio.reqif");
    assert_eq!(doc.objects.len(), 137);
    assert_eq!(result.imported_count, 0);
    assert!(
        !result.success,
        "the fixture contains a hierarchy reference to a missing SpecObject"
    );
    assert!(
        result
            .errors
            .iter()
            .any(|error| error.contains("_B9RbAGunEeuNUYnTveUm8Q")),
        "{:?}",
        result.errors
    );
}

#[test]
fn test_reqif_hierarchy() {
    let (doc, result) = import_fixture("strictdoc/strictdoc_04_sample2_sdoc.reqif");
    assert!(
        doc.hierarchy_edges.len() >= 10,
        "nested StrictDoc chapters should produce hierarchy edges, got {}",
        doc.hierarchy_edges.len()
    );
    assert_eq!(result.created_link_count, expected_link_count(&doc));
}

#[test]
fn test_reqif_attributes() {
    let (doc, _) = import_fixture("enterprise-architect/ea_example.reqif.xml");
    assert!(doc.objects.iter().any(|o| !o.attributes.is_empty()));
    let (_, _, description, status, _) = object_to_fields(&doc.objects[0]);
    assert_eq!(
        description.as_deref(),
        Some("The system shall observe the earths surface.")
    );
    assert!(
        status.is_some(),
        "vendor *-STATUS must map to Marreq status"
    );
}

#[test]
fn test_reqif_enumerations() {
    let (doc, result) = import_fixture("capella/eclipse_capella_Sample.xml");
    assert_eq!(
        doc.objects[0]
            .attributes
            .get("IE Object Type")
            .map(String::as_str),
        Some("Requirement")
    );
    assert_eq!(doc.enumeration_value_count, 1);
    assert!(result.warnings.iter().any(|w| w.contains("attribute")));
}

#[test]
fn test_reqif_xhtml() {
    let (doc, result) = import_fixture("polarion/polarion_export.xml");
    let (_, _, desc, _, _) = object_to_fields(&doc.objects[0]);
    assert!(
        desc.as_deref().unwrap_or_default().len() > 5,
        "XHTML THE-VALUE text should be captured"
    );
    assert!(result.success, "{:?}", result.errors);
}

#[test]
fn test_reqif_formatted_xhtml_tables_links_and_images() {
    let xml = load_fixture("synthetic/formatted-xhtml.reqif");
    let doc = parse_reqif(&xml).unwrap();
    let text = doc.objects[0]
        .attributes
        .get("ReqIF.Text")
        .expect("ReqIF.Text");
    for expected in [
        "Before",
        "linked text",
        "Cell A",
        "Cell B",
        "Image fallback",
    ] {
        assert!(text.contains(expected), "missing '{expected}' in '{text}'");
    }
    assert_eq!(doc.xhtml_value_count, 1);
    assert_eq!(doc.attachment_count, 1);
    assert!(doc
        .warnings
        .iter()
        .any(|warning| warning.contains("embedded")));
}

#[test]
fn test_reqif_integer_real_boolean_date_and_enumeration_values() {
    let xml = load_fixture("synthetic/typed-values.reqif");
    let doc = parse_reqif(&xml).unwrap();
    let attributes = &doc.objects[0].attributes;
    assert_eq!(
        attributes.get("Custom Integer").map(String::as_str),
        Some("42")
    );
    assert_eq!(
        attributes.get("Custom Real").map(String::as_str),
        Some("3.5")
    );
    assert_eq!(
        attributes.get("Custom Boolean").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        attributes.get("Custom Date").map(String::as_str),
        Some("2026-09-19T12:00:00Z")
    );
    assert_eq!(
        attributes.get("Custom Enum").map(String::as_str),
        Some("High")
    );
    assert_eq!(doc.scalar_value_count, 4);
    assert_eq!(doc.enumeration_value_count, 1);

    let (state, user, config) = blank_project();
    let result = ReqIFService::new(&state)
        .import_into_project(&xml, &config, &user)
        .unwrap();
    assert_eq!(result.imported_count, 1);
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.contains("typed")));
}

#[test]
fn test_reqif_relations() {
    let (doc, result) = import_fixture("capella/eclipse_capella_Sample1.xml");
    assert_eq!(doc.objects.len(), 2);
    assert_eq!(doc.relations.len(), 1);
    assert_eq!(result.created_link_count, 1);
}

#[test]
fn test_reqif_multiple_specifications() {
    let (doc, result) = import_fixture("eclipse-rmf/strictdoc_04_sample3_eclipse_rmf.reqif");
    assert_eq!(doc.specification_count, 2);
    assert_eq!(doc.objects.len(), 6);
    assert_eq!(result.imported_count, 6);
}

#[test]
fn test_reqif_multiple_object_types() {
    let (doc, _) = import_fixture("enterprise-architect/ea_example.reqif.xml");
    assert!(doc.object_type_count >= 2);
    let types: HashSet<_> = doc.objects.iter().map(|o| o.type_ref.clone()).collect();
    assert!(!types.is_empty());
}

#[test]
fn test_reqif_unknown_attributes() {
    let xml = load_fixture("synthetic/unknown-elements.reqif");
    let doc = parse_reqif(&xml).unwrap();
    assert_eq!(doc.objects.len(), 1);
}

#[test]
fn test_reqif_namespaces() {
    let xml = load_fixture("synthetic/prefixed-namespace.reqif");
    let doc = parse_reqif(&xml).unwrap();
    assert_eq!(doc.objects.len(), 1);
    assert_eq!(
        doc.objects[0]
            .attributes
            .get("ReqIF.Name")
            .map(String::as_str),
        Some("NS")
    );
}

#[test]
fn test_reqif_attachments() {
    let xml = load_fixture("synthetic/embedded-file.reqif");
    let doc = parse_reqif(&xml).unwrap();
    assert!(
        doc.warnings.iter().any(|w| w.contains("embedded")),
        "embedded content must be reported, not silently dropped without a warning: {:?}",
        doc.warnings
    );
}

#[test]
fn test_reqif_malformed_xml_does_not_panic() {
    let xml = load_fixture("synthetic/malformed.xml");
    assert!(parse_reqif(&xml).is_err());
}

#[test]
fn test_reqif_missing_relation_target_is_warned() {
    let xml = load_fixture("synthetic/dangling-relation.reqif");
    let (state, user, config) = blank_project();
    let service = ReqIFService::new(&state);
    let result = service.import_into_project(&xml, &config, &user).unwrap();
    assert_eq!(result.imported_count, 1);
    assert!(
        result
            .warnings
            .iter()
            .any(|error| error.contains("missing target")),
        "{:?}",
        result.warnings
    );
    assert!(
        RequirementService::new(&state)
            .list_by_project(1)
            .unwrap()
            .len()
            == 1,
        "valid objects should import even when an external relation target is absent"
    );
}

#[test]
fn test_reqif_reqifz_is_not_parsed_as_xml() {
    let bytes = load_fixture("polarion/polarion.reqifz");
    let err = parse_reqif(&bytes).unwrap_err();
    assert!(err.to_lowercase().contains("parse"));
}

#[test]
fn test_reqif_invalid_root_is_rejected() {
    let xml = load_fixture("synthetic/invalid-root.xml");
    assert!(parse_reqif(&xml).unwrap_err().contains("root element"));
}

#[test]
fn test_reqif_duplicate_identifiers_are_rejected_before_writes() {
    let xml = load_fixture("synthetic/duplicate-identifiers.reqif");
    let (state, user, config) = blank_project();
    let result = ReqIFService::new(&state)
        .import_into_project(&xml, &config, &user)
        .unwrap();
    assert!(!result.success);
    assert_eq!(result.imported_count, 0);
    assert!(result
        .errors
        .iter()
        .any(|error| error.contains("duplicate SPEC-OBJECT")));
    assert!(RequirementService::new(&state)
        .list_by_project(1)
        .unwrap()
        .is_empty());
}

#[test]
fn test_reqif_generated_references_do_not_collide_with_mapped_references() {
    let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
    <REQ-IF>
      <CORE-CONTENT><REQ-IF-CONTENT>
        <SPEC-TYPES><SPEC-OBJECT-TYPE IDENTIFIER="type">
          <SPEC-ATTRIBUTES>
            <ATTRIBUTE-DEFINITION-STRING IDENTIFIER="title" LONG-NAME="Title"/>
            <ATTRIBUTE-DEFINITION-STRING IDENTIFIER="reference" LONG-NAME="Identifier"/>
          </SPEC-ATTRIBUTES>
        </SPEC-OBJECT-TYPE></SPEC-TYPES>
        <SPEC-OBJECTS>
          <SPEC-OBJECT IDENTIFIER="uuid-one">
            <VALUES><ATTRIBUTE-VALUE-STRING DEFINITION="title" THE-VALUE="Generated"/></VALUES>
          </SPEC-OBJECT>
          <SPEC-OBJECT IDENTIFIER="uuid-two">
            <VALUES>
              <ATTRIBUTE-VALUE-STRING DEFINITION="title" THE-VALUE="Mapped"/>
              <ATTRIBUTE-VALUE-STRING DEFINITION="reference" THE-VALUE="REQ-0001"/>
            </VALUES>
          </SPEC-OBJECT>
        </SPEC-OBJECTS>
      </REQ-IF-CONTENT></CORE-CONTENT>
    </REQ-IF>"#;
    let (state, user, config) = blank_project();
    let result = ReqIFService::new(&state)
        .import_into_project(xml, &config, &user)
        .unwrap();
    assert!(result.success, "{:?}", result.errors);
    let references: HashSet<_> = RequirementService::new(&state)
        .list_by_project(1)
        .unwrap()
        .into_iter()
        .map(|requirement| requirement.reference_code)
        .collect();
    assert_eq!(
        references,
        HashSet::from(["REQ-0001".to_string(), "REQ-0002".to_string()])
    );
}

#[test]
fn test_reqif_round_trip_core_fields() {
    let xml = load_fixture("synthetic/marreq-style.reqif");
    let (state, user, config) = blank_project();
    let service = ReqIFService::new(&state);
    let imported = service.import_into_project(&xml, &config, &user).unwrap();
    assert_eq!(imported.imported_count, 2);
    assert_eq!(imported.created_link_count, 1);
    let exported = service.export_project(1).unwrap();
    let reparsed = parse_reqif(exported.as_bytes()).unwrap();
    assert_eq!(reparsed.objects.len(), 2);
    let titles: HashSet<_> = reparsed
        .objects
        .iter()
        .filter_map(|o| object_to_fields(o).0)
        .collect();
    assert!(titles.contains("Parent"));
    assert!(titles.contains("Child"));
    assert_eq!(reparsed.relations.len(), 1);
}

#[test]
fn test_reqif_vendor_fixture_semantic_matrix() {
    let fixtures = [
        ("capella/eclipse_capella_Sample.xml", 1, 1),
        ("capella/eclipse_capella_Sample1.xml", 2, 2),
        ("capella/eclipse_capella_Sample3.xml", 1, 1),
        ("doors/capella_Sample.reqif", 1, 1),
        ("doors/strictdoc_doors_date.reqif", 1, 1),
        ("doors/strictdoc_doors_user.reqif", 1, 1),
        ("eclipse-rmf/eclipse_rmf_sample.reqif", 2, 2),
        ("eclipse-rmf/eclipse_rmf_specRelationTest.reqif", 2, 2),
        ("eclipse-rmf/strictdoc_04_sample3_eclipse_rmf.reqif", 6, 6),
        ("enterprise-architect/ea_example.reqif.xml", 3, 0),
        ("enterprise-architect/strictdoc_ea8.reqif", 3, 0),
        (
            "polarion/antcc_MAG8000-LTE-FeatureSpecReqBL4.reqif",
            102,
            102,
        ),
        ("polarion/polarion_export.xml", 1, 1),
        ("polarion/strictdoc_04_sample1_polarion.reqif", 2, 2),
        ("polarion/strictdoc_polarion_anonymized.reqif", 101, 101),
        ("reqif-studio/strictdoc_reqif_studio.reqif", 137, 0),
        ("strictdoc/strictdoc_01_minimal_reqif_sample.reqif", 0, 0),
        ("strictdoc/strictdoc_02_read_reqif_input.reqif", 3, 3),
        ("strictdoc/strictdoc_04_sample2_sdoc.reqif", 18, 18),
    ];

    for (fixture, expected_objects, expected_imports) in fixtures {
        let (doc, result) = import_fixture(fixture);
        assert_eq!(doc.objects.len(), expected_objects, "{fixture}");
        assert_eq!(
            result.imported_count, expected_imports,
            "{fixture}: {:?}",
            result.errors
        );
        println!(
            "{fixture}\tobjects={}\thierarchy={}\trelations={}\tlinks={}\twarnings={}\terrors={}\tsuccess={}",
            doc.objects.len(),
            doc.hierarchy_edges.len(),
            doc.relations.len(),
            result.created_link_count,
            result.warnings.len(),
            result.errors.len(),
            result.success
        );
    }
}
