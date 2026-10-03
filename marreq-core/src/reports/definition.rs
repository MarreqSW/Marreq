// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Report template definitions: which sections a report has, in which order,
//! with which options, plus the document fields (ID, issue, signatories…).

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::sections::{self, OptionKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportType {
    /// Verification Control Document (ECSS-E-ST-10-02C Annex B).
    Vcd,
    /// Traceability & coverage report.
    Coverage,
}

impl ReportType {
    pub const ALL: [ReportType; 2] = [ReportType::Vcd, ReportType::Coverage];

    pub fn key(self) -> &'static str {
        match self {
            ReportType::Vcd => "vcd",
            ReportType::Coverage => "coverage",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            ReportType::Vcd => "Verification Control Document",
            ReportType::Coverage => "Traceability & Coverage Report",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            ReportType::Vcd => {
                "Per requirement: verification method, level and stage, evidence, compliance and close-out (ECSS-E-ST-10-02C Annex B)."
            }
            ReportType::Coverage => {
                "Coverage KPIs, coverage by group, uncovered requirements, orphan verifications, suspect links and data quality."
            }
        }
    }

    pub fn parse(key: &str) -> Option<Self> {
        ReportType::ALL.into_iter().find(|t| t.key() == key)
    }

    /// Short code used in default document IDs.
    pub fn doc_code(self) -> &'static str {
        match self {
            ReportType::Vcd => "VCD",
            ReportType::Coverage => "TCR",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageSize {
    #[default]
    A4,
    Letter,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignatoryRow {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeRow {
    #[serde(default)]
    pub issue: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub changes: String,
    #[serde(default)]
    pub author: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentRow {
    /// e.g. "AD1", "RD2".
    #[serde(default, rename = "ref")]
    pub reference: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub issue: String,
}

/// Fields printed on the cover, in the running header and in the front matter.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentSettings {
    #[serde(default)]
    pub doc_id: String,
    /// Empty: the report type's title.
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub issue: String,
    #[serde(default)]
    pub revision: String,
    #[serde(default)]
    pub classification: String,
    /// Diagonal text on every page (e.g. "DRAFT"); PDF only.
    #[serde(default)]
    pub watermark: Option<String>,
    #[serde(default)]
    pub page_size: PageSize,
    /// Archival PDF/A-2b; PDF only.
    #[serde(default)]
    pub pdf_a: bool,
    #[serde(default)]
    pub signatories: Vec<SignatoryRow>,
    #[serde(default)]
    pub change_record: Vec<ChangeRow>,
    #[serde(default)]
    pub documents: Vec<DocumentRow>,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SectionConfig {
    pub key: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub options: Map<String, Value>,
}

/// A report template. The order of `sections` is the order in the document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportDefinition {
    pub report_type: ReportType,
    #[serde(default)]
    pub document: DocumentSettings,
    #[serde(default)]
    pub sections: Vec<SectionConfig>,
}

/// Values the built-in defaults are made from.
#[derive(Debug, Clone)]
pub struct DefaultContext {
    pub project_name: String,
    pub project_slug: String,
    pub user_name: String,
    /// `YYYY-MM-DD`.
    pub today: String,
}

const SHORT: usize = 200;
const LONG: usize = 4000;
const MAX_ROWS: usize = 50;

fn check_len(field: &str, value: &str, max: usize) -> Result<(), String> {
    if value.chars().count() > max {
        return Err(format!("{field} must be at most {max} characters"));
    }
    Ok(())
}

impl ReportDefinition {
    /// The built-in template of a report type: every section in its default
    /// order and state, with document fields filled from the project.
    pub fn default_for(report_type: ReportType, ctx: &DefaultContext) -> Self {
        let sections = sections::catalog()
            .into_iter()
            .filter(|s| s.report_types.contains(&report_type))
            .map(|s| SectionConfig {
                key: s.key.to_string(),
                enabled: s.default_enabled,
                options: Map::new(),
            })
            .collect();
        let slug = ctx
            .project_slug
            .to_uppercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>();
        let documents = match report_type {
            ReportType::Vcd => vec![
                DocumentRow {
                    reference: "AD1".into(),
                    id: "ECSS-E-ST-10-02C Rev.1".into(),
                    title: "Space engineering — Verification".into(),
                    issue: "1 February 2018".into(),
                },
                DocumentRow {
                    reference: "AD2".into(),
                    id: format!("{slug}-SRS"),
                    title: format!("{} requirements specification", ctx.project_name),
                    issue: String::new(),
                },
            ],
            ReportType::Coverage => Vec::new(),
        };
        ReportDefinition {
            report_type,
            document: DocumentSettings {
                doc_id: format!("{slug}-{}-001", report_type.doc_code()),
                title: String::new(),
                issue: "1".into(),
                revision: "0".into(),
                classification: String::new(),
                watermark: None,
                page_size: PageSize::A4,
                pdf_a: false,
                signatories: vec![
                    SignatoryRow {
                        role: "Prepared by".into(),
                        name: ctx.user_name.clone(),
                    },
                    SignatoryRow {
                        role: "Reviewed by".into(),
                        name: String::new(),
                    },
                    SignatoryRow {
                        role: "Approved by".into(),
                        name: String::new(),
                    },
                ],
                change_record: vec![ChangeRow {
                    issue: "1.0".into(),
                    date: ctx.today.clone(),
                    changes: "First issue".into(),
                    author: ctx.user_name.clone(),
                }],
                documents,
            },
            sections,
        }
    }

    /// Check the definition and complete it: every section of the report
    /// type must be known, listed at most once, with valid options. Sections
    /// the definition does not list (e.g. added in a later release) are
    /// appended **disabled**, so a saved template keeps producing the same
    /// document.
    pub fn validated(mut self) -> Result<Self, String> {
        let catalog: Vec<_> = sections::catalog()
            .into_iter()
            .filter(|s| s.report_types.contains(&self.report_type))
            .collect();
        let mut seen = HashSet::new();
        for section in &self.sections {
            let Some(spec) = catalog.iter().find(|s| s.key == section.key) else {
                return Err(format!(
                    "unknown section '{}' for report type {}",
                    section.key,
                    self.report_type.key()
                ));
            };
            if !seen.insert(section.key.clone()) {
                return Err(format!("section '{}' is listed twice", section.key));
            }
            for (name, value) in &section.options {
                let Some(option) = spec.options.iter().find(|o| o.key == name) else {
                    return Err(format!(
                        "unknown option '{name}' for section '{}'",
                        section.key
                    ));
                };
                option.kind.check(name, value)?;
            }
        }
        for spec in &catalog {
            if !seen.contains(spec.key) {
                self.sections.push(SectionConfig {
                    key: spec.key.to_string(),
                    enabled: false,
                    options: Map::new(),
                });
            }
        }

        let d = &self.document;
        for (field, value, max) in [
            ("doc_id", d.doc_id.as_str(), 80),
            ("title", d.title.as_str(), SHORT),
            ("issue", d.issue.as_str(), 20),
            ("revision", d.revision.as_str(), 20),
            ("classification", d.classification.as_str(), 80),
            ("watermark", d.watermark.as_deref().unwrap_or(""), 30),
        ] {
            check_len(field, value, max)?;
        }
        for (field, len) in [
            ("signatories", d.signatories.len()),
            ("change_record", d.change_record.len()),
            ("documents", d.documents.len()),
        ] {
            if len > MAX_ROWS {
                return Err(format!("{field} can have at most {MAX_ROWS} rows"));
            }
        }
        for s in &d.signatories {
            check_len("signatory role", &s.role, SHORT)?;
            check_len("signatory name", &s.name, SHORT)?;
        }
        for c in &d.change_record {
            check_len("change record issue", &c.issue, 20)?;
            check_len("change record date", &c.date, 40)?;
            check_len("change record changes", &c.changes, LONG)?;
            check_len("change record author", &c.author, SHORT)?;
        }
        for r in &d.documents {
            check_len("document ref", &r.reference, 20)?;
            check_len("document id", &r.id, SHORT)?;
            check_len("document title", &r.title, SHORT)?;
            check_len("document issue", &r.issue, 40)?;
        }
        Ok(self)
    }

    /// Enabled sections in document order.
    pub fn enabled_sections(&self) -> impl Iterator<Item = &SectionConfig> {
        self.sections.iter().filter(|s| s.enabled)
    }
}

impl OptionKind {
    fn check(&self, name: &str, value: &Value) -> Result<(), String> {
        match self {
            OptionKind::Select { choices } => match value.as_str() {
                Some(v) if choices.iter().any(|c| c.value == v) => Ok(()),
                _ => Err(format!(
                    "option '{name}' must be one of: {}",
                    choices
                        .iter()
                        .map(|c| c.value)
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            },
            OptionKind::MultiSelect { choices } => {
                let Some(items) = value.as_array() else {
                    return Err(format!("option '{name}' must be a list"));
                };
                let mut seen = HashSet::new();
                for item in items {
                    match item.as_str() {
                        Some(v) if choices.iter().any(|c| c.value == v) => {
                            if !seen.insert(v) {
                                return Err(format!("option '{name}' lists '{v}' twice"));
                            }
                        }
                        _ => {
                            return Err(format!(
                                "option '{name}' accepts only: {}",
                                choices
                                    .iter()
                                    .map(|c| c.value)
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ));
                        }
                    }
                }
                if items.is_empty() {
                    return Err(format!("option '{name}' needs at least one value"));
                }
                Ok(())
            }
            OptionKind::Text { .. } => match value.as_str() {
                Some(v) => check_len(&format!("option '{name}'"), v, LONG),
                None => Err(format!("option '{name}' must be text")),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx() -> DefaultContext {
        DefaultContext {
            project_name: "Satellite Demo".into(),
            project_slug: "satellite-demo".into(),
            user_name: "Alice".into(),
            today: "2026-10-03".into(),
        }
    }

    fn parse(v: Value) -> Result<ReportDefinition, String> {
        serde_json::from_value::<ReportDefinition>(v)
            .map_err(|e| e.to_string())?
            .validated()
    }

    #[test]
    fn defaults_list_every_section_of_their_type_and_validate() {
        for t in ReportType::ALL {
            let d = ReportDefinition::default_for(t, &ctx());
            let keys: Vec<_> = d.sections.iter().map(|s| s.key.as_str()).collect();
            assert!(
                keys.starts_with(&["cover", "change_record", "toc"]),
                "{keys:?}"
            );
            assert_eq!(
                d.clone().validated().unwrap(),
                d,
                "defaults are already complete"
            );
            assert_eq!(
                d.document.doc_id,
                format!("SATELLITE-DEMO-{}-001", t.doc_code())
            );
            assert_eq!(d.document.signatories[0].name, "Alice");
        }
        let vcd = ReportDefinition::default_for(ReportType::Vcd, &ctx());
        assert!(vcd.sections.iter().any(|s| s.key == "matrix"));
        assert!(
            !vcd.sections
                .iter()
                .any(|s| s.key == "uncovered_requirements")
        );
    }

    #[test]
    fn order_is_kept_and_missing_sections_are_appended_disabled() {
        let d = parse(json!({
            "report_type": "vcd",
            "sections": [
                {"key": "matrix", "options": {"group_by": "category", "columns": ["id", "closeout"]}},
                {"key": "cover", "enabled": false}
            ]
        }))
        .unwrap();
        assert_eq!(d.sections[0].key, "matrix");
        assert!(d.sections[0].enabled, "enabled defaults to true");
        assert_eq!(d.sections[1].key, "cover");
        assert!(!d.sections[1].enabled);
        assert!(d.sections.len() > 2);
        assert!(
            d.sections[2..].iter().all(|s| !s.enabled),
            "appended sections are off"
        );
        let enabled: Vec<_> = d.enabled_sections().map(|s| s.key.as_str()).collect();
        assert_eq!(enabled, ["matrix"]);
    }

    #[test]
    fn invalid_definitions_are_rejected_with_a_reason() {
        let cases = [
            (
                json!({"report_type": "vcd", "sections": [{"key": "nope"}]}),
                "unknown section 'nope'",
            ),
            (
                json!({"report_type": "coverage", "sections": [{"key": "matrix"}]}),
                "unknown section 'matrix' for report type coverage",
            ),
            (
                json!({"report_type": "vcd", "sections": [{"key": "toc"}, {"key": "toc"}]}),
                "listed twice",
            ),
            (
                json!({"report_type": "vcd", "sections": [{"key": "summary", "options": {"group_by": "colour"}}]}),
                "must be one of",
            ),
            (
                json!({"report_type": "vcd", "sections": [{"key": "summary", "options": {"size": 3}}]}),
                "unknown option 'size'",
            ),
            (
                json!({"report_type": "vcd", "sections": [{"key": "matrix", "options": {"columns": []}}]}),
                "at least one",
            ),
            (
                json!({"report_type": "vcd", "sections": [{"key": "matrix", "options": {"columns": ["id", "id"]}}]}),
                "twice",
            ),
            (
                json!({"report_type": "vcd", "document": {"doc_id": "x".repeat(81)}}),
                "doc_id must be at most 80",
            ),
        ];
        for (value, expected) in cases {
            let err = parse(value.clone()).expect_err(&value.to_string());
            assert!(err.contains(expected), "{value}: {err}");
        }
        assert!(
            parse(json!({"report_type": "vcd", "colour": 1})).is_err(),
            "unknown fields"
        );
        assert!(
            parse(json!({"report_type": "pie"})).is_err(),
            "unknown type"
        );
    }
}
