// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! The section catalog (what a template may contain) and the renderers that
//! turn [`ReportData`] into document blocks. To add a section: add a
//! [`SectionSpec`] to [`catalog`], a match arm in [`render_section`], and a
//! test.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value, json};

use super::data::{ReportData, ReqRow, code_prefix};
use super::definition::{ReportDefinition, ReportType};
use super::model::{
    BarPart, Block, Cell, Column, Inline, KeyValue, Kpi, Numbering, Signatory, Table, Tone,
};
use crate::services::verification_control_service::CloseOutStatus;
use crate::status_enums::VerificationOutcome;

#[derive(Debug, Clone, Serialize)]
pub struct Choice {
    pub value: &'static str,
    pub label: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OptionKind {
    Select { choices: Vec<Choice> },
    MultiSelect { choices: Vec<Choice> },
    Text { multiline: bool },
}

#[derive(Debug, Clone, Serialize)]
pub struct OptionSpec {
    pub key: &'static str,
    pub label: &'static str,
    #[serde(flatten)]
    pub kind: OptionKind,
    pub default: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct SectionSpec {
    pub key: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub report_types: Vec<ReportType>,
    pub default_enabled: bool,
    pub options: Vec<OptionSpec>,
}

const BOTH: [ReportType; 2] = [ReportType::Vcd, ReportType::Coverage];

fn group_by_option(with_none: bool) -> OptionSpec {
    let mut choices = vec![
        Choice {
            value: "code_prefix",
            label: "Reference code prefix",
        },
        Choice {
            value: "category",
            label: "Category",
        },
    ];
    if with_none {
        choices.push(Choice {
            value: "none",
            label: "No grouping",
        });
    }
    OptionSpec {
        key: "group_by",
        label: "Group by",
        kind: OptionKind::Select { choices },
        default: json!("code_prefix"),
    }
}

pub const MATRIX_COLUMNS: [(&str, &str); 10] = [
    ("id", "Req. ID"),
    ("title", "Title"),
    ("text", "Statement"),
    ("parent", "Parent"),
    ("method", "Method"),
    ("level_stage", "Level / stage"),
    ("verifications", "Verifications"),
    ("evidence", "Evidence"),
    ("compliance", "Compliance"),
    ("closeout", "Close-out"),
];

/// Every section any report type can contain, in default order.
pub fn catalog() -> Vec<SectionSpec> {
    let vcd = vec![ReportType::Vcd];
    let cov = vec![ReportType::Coverage];
    vec![
        SectionSpec {
            key: "cover",
            title: "Cover page",
            description: "Title, document ID, issue, date, classification and sign-off table.",
            report_types: BOTH.to_vec(),
            default_enabled: true,
            options: vec![],
        },
        SectionSpec {
            key: "change_record",
            title: "Change record",
            description: "Issues of the document and what changed in each.",
            report_types: BOTH.to_vec(),
            default_enabled: true,
            options: vec![],
        },
        SectionSpec {
            key: "toc",
            title: "Contents",
            description: "Table of contents with page numbers.",
            report_types: BOTH.to_vec(),
            default_enabled: true,
            options: vec![],
        },
        SectionSpec {
            key: "introduction",
            title: "Introduction",
            description: "Purpose and scope of the document.",
            report_types: BOTH.to_vec(),
            default_enabled: true,
            options: vec![OptionSpec {
                key: "text",
                label: "Text (empty: the standard introduction)",
                kind: OptionKind::Text { multiline: true },
                default: json!(""),
            }],
        },
        SectionSpec {
            key: "documents",
            title: "Applicable and reference documents",
            description: "The documents listed in the document settings.",
            report_types: BOTH.to_vec(),
            default_enabled: true,
            options: vec![],
        },
        SectionSpec {
            key: "definitions",
            title: "Definitions and abbreviations",
            description: "Verification method codes, compliance and close-out terms.",
            report_types: vcd.clone(),
            default_enabled: true,
            options: vec![],
        },
        SectionSpec {
            key: "approach",
            title: "Verification approach",
            description: "Levels, stages and the close-out rule.",
            report_types: vcd.clone(),
            default_enabled: true,
            options: vec![],
        },
        SectionSpec {
            key: "summary",
            title: "Verification summary status",
            description: "Headline figures, compliance by group and close-out by method.",
            report_types: vcd.clone(),
            default_enabled: true,
            options: vec![group_by_option(false)],
        },
        SectionSpec {
            key: "matrix",
            title: "Verification control matrix",
            description: "One row per requirement, on landscape pages.",
            report_types: vcd.clone(),
            default_enabled: true,
            options: vec![
                group_by_option(true),
                OptionSpec {
                    key: "columns",
                    label: "Columns",
                    kind: OptionKind::MultiSelect {
                        choices: MATRIX_COLUMNS
                            .iter()
                            .map(|(value, label)| Choice { value, label })
                            .collect(),
                    },
                    default: json!(MATRIX_COLUMNS.iter().map(|(v, _)| *v).collect::<Vec<_>>()),
                },
            ],
        },
        SectionSpec {
            key: "open_items",
            title: "Open verification items",
            description: "Requirements that are not closed, with the reason.",
            report_types: vcd.clone(),
            default_enabled: true,
            options: vec![],
        },
        SectionSpec {
            key: "traceability_checks",
            title: "Traceability checks",
            description: "Uncovered requirements, orphan verifications, suspect links, drafts.",
            report_types: vcd,
            default_enabled: true,
            options: vec![],
        },
        SectionSpec {
            key: "coverage_summary",
            title: "Coverage summary",
            description: "Coverage figures and requirements / verifications by status.",
            report_types: cov.clone(),
            default_enabled: true,
            options: vec![],
        },
        SectionSpec {
            key: "coverage_by_group",
            title: "Coverage by group",
            description: "Share of requirements with a linked verification, per group.",
            report_types: cov.clone(),
            default_enabled: true,
            options: vec![group_by_option(false)],
        },
        SectionSpec {
            key: "uncovered_requirements",
            title: "Requirements without verification",
            description: "Requirements with no linked verification.",
            report_types: cov.clone(),
            default_enabled: true,
            options: vec![],
        },
        SectionSpec {
            key: "orphan_verifications",
            title: "Verifications without requirements",
            description: "Verifications not linked to any requirement.",
            report_types: cov.clone(),
            default_enabled: true,
            options: vec![],
        },
        SectionSpec {
            key: "suspect_links",
            title: "Suspect links",
            description: "Links flagged because the requirement changed after linking.",
            report_types: cov.clone(),
            default_enabled: true,
            options: vec![],
        },
        SectionSpec {
            key: "data_quality",
            title: "Data quality",
            description: "Missing codes, empty statements, unknown categories, broken parents.",
            report_types: cov.clone(),
            default_enabled: true,
            options: vec![],
        },
        SectionSpec {
            key: "approval_and_authors",
            title: "Approval and authorship",
            description: "Requirements by approval state and by author.",
            report_types: cov,
            default_enabled: false,
            options: vec![],
        },
    ]
}

/// Render the enabled sections of `def` in order.
pub fn render(data: &ReportData, def: &ReportDefinition, generated_on: &str) -> Vec<Block> {
    let mut ctx = Ctx {
        data,
        def,
        numbering: Numbering::new(),
        generated_on,
    };
    let mut blocks = Vec::new();
    for section in def.enabled_sections() {
        blocks.extend(render_section(&mut ctx, &section.key, &section.options));
    }
    blocks
}

struct Ctx<'a> {
    data: &'a ReportData,
    def: &'a ReportDefinition,
    numbering: Numbering,
    generated_on: &'a str,
}

impl Ctx<'_> {
    fn heading(&mut self, level: u8, text: &str) -> Block {
        Block::Heading {
            level,
            number: self.numbering.next(level),
            text: text.into(),
            outlined: true,
        }
    }

    fn title(&self) -> String {
        if self.def.document.title.trim().is_empty() {
            self.def.report_type.title().to_string()
        } else {
            self.def.document.title.trim().to_string()
        }
    }
}

fn opt_str<'a>(options: &'a Map<String, Value>, key: &str, default: &'a str) -> &'a str {
    options.get(key).and_then(Value::as_str).unwrap_or(default)
}

fn pct(part: usize, whole: usize) -> String {
    if whole == 0 {
        "0%".into()
    } else {
        format!("{}%", ((part as f64 / whole as f64) * 100.0).round() as i64)
    }
}

fn unnumbered(text: &str) -> Block {
    Block::Heading {
        level: 1,
        number: String::new(),
        text: text.into(),
        outlined: false,
    }
}

fn none_paragraph() -> Block {
    Block::para(vec![Inline::muted("None.")])
}

fn compliance_tone(c: Option<&str>) -> Tone {
    match c {
        Some("C") => Tone::Ok,
        Some("PC") => Tone::Warn,
        Some("NC") => Tone::Bad,
        _ => Tone::None,
    }
}

fn compliance_badge(r: &ReqRow) -> Inline {
    let label = r.compliance.clone().unwrap_or_else(|| "–".into());
    Inline::badge(compliance_tone(r.compliance.as_deref()), label)
}

fn outcome_tone(o: VerificationOutcome) -> Tone {
    match o {
        VerificationOutcome::Passed => Tone::Ok,
        VerificationOutcome::Failed => Tone::Bad,
        VerificationOutcome::InProgress => Tone::Open,
        VerificationOutcome::NotRun => Tone::None,
    }
}

fn closeout_badge(r: &ReqRow) -> Inline {
    match r.close_out.status {
        CloseOutStatus::Closed => Inline::badge(Tone::Ok, "Closed"),
        CloseOutStatus::Open => Inline::badge(Tone::Open, "Open"),
    }
}

/// Assessment buckets used by the summaries: C, PC, NC, not assessed.
#[derive(Default)]
struct Buckets {
    total: usize,
    covered: usize,
    c: usize,
    pc: usize,
    nc: usize,
    unassessed: usize,
    closed: usize,
}

impl Buckets {
    fn add(&mut self, r: &ReqRow) {
        self.total += 1;
        if !r.verifications.is_empty() {
            self.covered += 1;
        }
        match r.compliance.as_deref() {
            Some("C") => self.c += 1,
            Some("PC") => self.pc += 1,
            Some("NC") => self.nc += 1,
            _ => self.unassessed += 1,
        }
        if r.is_closed() {
            self.closed += 1;
        }
    }

    fn bar(&self) -> Cell {
        Cell::Bar {
            parts: vec![
                BarPart {
                    value: self.c as f64,
                    tone: Tone::Ok,
                },
                BarPart {
                    value: self.pc as f64,
                    tone: Tone::Warn,
                },
                BarPart {
                    value: self.nc as f64,
                    tone: Tone::Bad,
                },
                BarPart {
                    value: self.unassessed as f64,
                    tone: Tone::None,
                },
            ],
        }
    }
}

fn group_of(r: &ReqRow, group_by: &str) -> String {
    match group_by {
        "category" => r.category.clone(),
        "none" => String::new(),
        _ => code_prefix(&r.code),
    }
}

/// Requirements grouped (BTreeMap: groups in name order, rows in code order).
fn grouped<'a>(data: &'a ReportData, group_by: &str) -> BTreeMap<String, Vec<&'a ReqRow>> {
    let mut groups: BTreeMap<String, Vec<&ReqRow>> = BTreeMap::new();
    for r in &data.requirements {
        groups.entry(group_of(r, group_by)).or_default().push(r);
    }
    groups
}

fn render_section(ctx: &mut Ctx, key: &str, options: &Map<String, Value>) -> Vec<Block> {
    let data = ctx.data;
    match key {
        "cover" => cover(ctx),
        "change_record" => {
            let mut out = vec![unnumbered("Change record")];
            let rows = &ctx.def.document.change_record;
            if rows.is_empty() {
                out.push(none_paragraph());
            } else {
                let mut t = Table::new(vec![
                    Column::new("Issue", 0.1),
                    Column::new("Date", 0.16),
                    Column::new("Changes", 0.54),
                    Column::new("Author", 0.2),
                ]);
                for r in rows {
                    t.row(vec![
                        Cell::text(&r.issue),
                        Cell::text(&r.date),
                        Cell::text(&r.changes),
                        Cell::text(&r.author),
                    ]);
                }
                out.push(Block::Table(t));
            }
            out
        }
        "toc" => vec![
            Block::Toc {
                title: "Contents".into(),
            },
            Block::PageBreak,
        ],
        "introduction" => {
            let text = opt_str(options, "text", "").trim().to_string();
            let mut out = vec![ctx.heading(1, "Introduction")];
            if text.is_empty() {
                out.extend(default_introduction(ctx));
            } else {
                out.extend(text.split("\n\n").map(|p| Block::text(p.trim())));
            }
            out
        }
        "documents" => {
            let mut out = vec![ctx.heading(1, "Applicable and reference documents")];
            let rows = &ctx.def.document.documents;
            if rows.is_empty() {
                out.push(none_paragraph());
            } else {
                let mut t = Table::new(vec![
                    Column::new("Ref.", 0.08),
                    Column::new("Document", 0.27),
                    Column::new("Title", 0.47),
                    Column::new("Issue", 0.18),
                ]);
                for r in rows {
                    t.row(vec![
                        Cell::text(&r.reference),
                        Cell::text(&r.id),
                        Cell::text(&r.title),
                        Cell::text(&r.issue),
                    ]);
                }
                out.push(Block::Table(t));
            }
            out
        }
        "definitions" => {
            let mut t = Table::new(vec![
                Column::new("Term", 0.25),
                Column::new("Meaning", 0.75),
            ]);
            let methods = if data.method_legend.is_empty() {
                "T Test · A Analysis · R Review of design · I Inspection".to_string()
            } else {
                data.method_legend
                    .iter()
                    .map(|(l, t)| format!("{l} {t}"))
                    .collect::<Vec<_>>()
                    .join(" · ")
            };
            for (term, meaning) in [
                ("Methods", methods.as_str()),
                (
                    "C / PC / NC",
                    "Compliance: compliant, partially compliant (accepted, e.g. with a waiver), non-compliant",
                ),
                (
                    "Closed / Open",
                    "Close-out: verification complete and accepted / verification outstanding",
                ),
                (
                    "Level",
                    "Where the requirement is verified, e.g. equipment, subsystem, system",
                ),
                (
                    "Stage",
                    "When it is verified, e.g. QUAL (qualification), ACC (acceptance)",
                ),
                (
                    "Evidence",
                    "Reference of the document proving the result (test report, analysis…)",
                ),
            ] {
                t.row(vec![
                    Cell::of(vec![Inline::bold(term)]),
                    Cell::text(meaning),
                ]);
            }
            vec![
                ctx.heading(1, "Definitions and abbreviations"),
                Block::Table(t),
            ]
        }
        "approach" => vec![
            ctx.heading(1, "Verification approach"),
            Block::text(
                "Each requirement is verified with the method recorded on it, at the level and stage recorded on its verifications. A requirement is closed when:",
            ),
            Block::Bullets {
                items: vec![
                    vec![Inline::text("at least one verification is linked to it,")],
                    vec![Inline::text("every linked verification has passed, and")],
                    vec![Inline::text(
                        "a project reviewer assessed it as compliant (C) or partially compliant (PC).",
                    )],
                ],
            },
            Block::text(
                "Otherwise it stays open, and the matrix gives the reason: no verification, a failed or unfinished verification, no assessment yet, or non-compliance.",
            ),
        ],
        "summary" => vcd_summary(ctx, opt_str(options, "group_by", "code_prefix")),
        "matrix" => vcd_matrix(ctx, options),
        "open_items" => {
            let open: Vec<&ReqRow> = data
                .requirements
                .iter()
                .filter(|r| !r.is_closed())
                .collect();
            let mut out = vec![
                ctx.heading(1, "Open verification items"),
                Block::text(format!(
                    "{} of {} requirements are not closed.",
                    open.len(),
                    data.requirements.len()
                )),
            ];
            if !open.is_empty() {
                let mut t = Table::new(vec![
                    Column::new("Req. ID", 0.14),
                    Column::new("Requirement", 0.3),
                    Column::new("Method", 0.08).center(),
                    Column::new("Compl.", 0.08).center(),
                    Column::new("Reason", 0.25),
                    Column::new("Verifications", 0.15),
                ]);
                t.compact = true;
                for r in open {
                    t.row(vec![
                        Cell::of(vec![Inline::code(r.label())]),
                        Cell::text(&r.title),
                        Cell::text(r.methods.join(", ")),
                        Cell::of(vec![compliance_badge(r)]),
                        Cell::text(&r.close_out.reason),
                        Cell::text(verification_codes(data, r)),
                    ]);
                }
                out.push(Block::Table(t));
            }
            out
        }
        "traceability_checks" => {
            let uncovered = data
                .requirements
                .iter()
                .filter(|r| r.verifications.is_empty())
                .count();
            let orphans = data
                .verifications
                .iter()
                .filter(|v| v.requirement_ids.is_empty())
                .count();
            let suspect = data.links.iter().filter(|l| l.suspect).count();
            let drafts = data
                .requirements
                .iter()
                .filter(|r| r.approval_state == "draft")
                .count();
            let count = |n: usize, tone_if_any: Tone| {
                if n == 0 {
                    Cell::of(vec![Inline::badge(Tone::Ok, "none")])
                } else {
                    Cell::of(vec![Inline::badge(tone_if_any, n.to_string())])
                }
            };
            let mut t = Table::new(vec![Column::new("Check", 0.7), Column::new("Result", 0.3)]);
            t.row(vec![
                Cell::text("Requirements without a linked verification"),
                count(uncovered, Tone::Bad),
            ]);
            t.row(vec![
                Cell::text("Verifications not linked to a requirement"),
                count(orphans, Tone::Warn),
            ]);
            t.row(vec![
                Cell::text("Suspect links (requirement changed after linking)"),
                count(suspect, Tone::Warn),
            ]);
            t.row(vec![
                Cell::text("Requirements still in draft"),
                count(drafts, Tone::Warn),
            ]);
            vec![
                ctx.heading(1, "Traceability checks"),
                Block::text(format!(
                    "Checks run on the live traceability data on {}.",
                    ctx.generated_on
                )),
                Block::Table(t),
            ]
        }
        "coverage_summary" => coverage_summary(ctx),
        "coverage_by_group" => {
            let group_by = opt_str(options, "group_by", "code_prefix");
            let label = if group_by == "category" {
                "category"
            } else {
                "code prefix"
            };
            let mut t = Table::new(vec![
                Column::new(
                    if group_by == "category" {
                        "Category"
                    } else {
                        "Prefix"
                    },
                    0.3,
                ),
                Column::new("Requirements", 0.14).right(),
                Column::new("Covered", 0.14).right(),
                Column::new("Coverage", 0.12).right(),
                Column::new("", 0.3),
            ]);
            for (group, rows) in grouped(data, group_by) {
                let covered = rows.iter().filter(|r| !r.verifications.is_empty()).count();
                t.row(vec![
                    Cell::text(group),
                    Cell::text(rows.len().to_string()),
                    Cell::text(covered.to_string()),
                    Cell::text(pct(covered, rows.len())),
                    Cell::Bar {
                        parts: vec![
                            BarPart {
                                value: covered as f64,
                                tone: Tone::Ok,
                            },
                            BarPart {
                                value: (rows.len() - covered) as f64,
                                tone: Tone::None,
                            },
                        ],
                    },
                ]);
            }
            vec![
                ctx.heading(1, &format!("Coverage by {label}")),
                Block::Table(t),
            ]
        }
        "uncovered_requirements" => {
            let rows: Vec<&ReqRow> = data
                .requirements
                .iter()
                .filter(|r| r.verifications.is_empty())
                .collect();
            let mut out = vec![ctx.heading(1, "Requirements without verification")];
            if rows.is_empty() {
                out.push(Block::para(vec![Inline::badge(
                    Tone::Ok,
                    "Every requirement has a linked verification.",
                )]));
            } else {
                let mut t = Table::new(vec![
                    Column::new("Req. ID", 0.16),
                    Column::new("Title", 0.5),
                    Column::new("Category", 0.17),
                    Column::new("Status", 0.17),
                ]);
                for r in rows {
                    t.row(vec![
                        Cell::of(vec![Inline::code(r.label())]),
                        Cell::text(&r.title),
                        Cell::text(&r.category),
                        Cell::text(&r.status),
                    ]);
                }
                out.push(Block::Table(t));
            }
            out
        }
        "orphan_verifications" => {
            let rows: Vec<_> = data
                .verifications
                .iter()
                .filter(|v| v.requirement_ids.is_empty())
                .collect();
            let mut out = vec![ctx.heading(1, "Verifications without requirements")];
            if rows.is_empty() {
                out.push(Block::para(vec![Inline::badge(
                    Tone::Ok,
                    "Every verification is linked to a requirement.",
                )]));
            } else {
                let mut t = Table::new(vec![
                    Column::new("Ver. ID", 0.16),
                    Column::new("Name", 0.6),
                    Column::new("Status", 0.24),
                ]);
                for v in rows {
                    t.row(vec![
                        Cell::of(vec![Inline::code(v.label())]),
                        Cell::text(&v.name),
                        Cell::of(vec![Inline::badge(outcome_tone(v.outcome), &v.status)]),
                    ]);
                }
                out.push(Block::Table(t));
            }
            out
        }
        "suspect_links" => {
            let rows: Vec<_> = data.links.iter().filter(|l| l.suspect).collect();
            let mut out = vec![ctx.heading(1, "Suspect links")];
            if rows.is_empty() {
                out.push(Block::para(vec![Inline::badge(
                    Tone::Ok,
                    "No suspect links.",
                )]));
            } else {
                let mut t = Table::new(vec![
                    Column::new("Requirement", 0.2),
                    Column::new("Verification", 0.2),
                    Column::new("Reason", 0.6),
                ]);
                for l in rows {
                    let r = data.requirement(l.req_id).map(|r| r.label().to_string());
                    let v = data.verification(l.ver_id).map(|v| v.label().to_string());
                    t.row(vec![
                        Cell::of(vec![Inline::code(
                            r.unwrap_or_else(|| format!("#{}", l.req_id)),
                        )]),
                        Cell::of(vec![Inline::code(
                            v.unwrap_or_else(|| format!("#{}", l.ver_id)),
                        )]),
                        Cell::text(l.suspect_reason.clone().unwrap_or_default()),
                    ]);
                }
                out.push(Block::Table(t));
            }
            out
        }
        "data_quality" => data_quality(ctx),
        "approval_and_authors" => {
            let mut by_state: BTreeMap<&str, usize> = BTreeMap::new();
            let mut by_author: BTreeMap<&str, usize> = BTreeMap::new();
            for r in &data.requirements {
                *by_state.entry(r.approval_state.as_str()).or_default() += 1;
                *by_author.entry(r.author.as_str()).or_default() += 1;
            }
            let table = |title: &str, rows: BTreeMap<&str, usize>| {
                let mut t = Table::new(vec![
                    Column::new(title, 0.7),
                    Column::new("Requirements", 0.3).right(),
                ]);
                for (k, n) in rows {
                    t.row(vec![Cell::text(k), Cell::text(n.to_string())]);
                }
                Block::Table(t)
            };
            vec![
                ctx.heading(1, "Approval and authorship"),
                ctx.heading(2, "By approval state"),
                table("Approval state", by_state),
                ctx.heading(2, "By author"),
                table("Author", by_author),
            ]
        }
        // `validated()` only lets catalog keys through.
        _ => Vec::new(),
    }
}

fn cover(ctx: &mut Ctx) -> Vec<Block> {
    let d = &ctx.def.document;
    let issue = match (d.issue.trim(), d.revision.trim()) {
        ("", "") => String::new(),
        (i, "") => format!("Issue {i}"),
        ("", r) => format!("Rev {r}"),
        (i, r) => format!("Issue {i} · Rev {r}"),
    };
    let mut fields = vec![
        KeyValue {
            key: "Document".into(),
            value: d.doc_id.clone(),
        },
        KeyValue {
            key: "Issue".into(),
            value: issue,
        },
        KeyValue {
            key: "Date".into(),
            value: ctx.generated_on.into(),
        },
        KeyValue {
            key: "Project".into(),
            value: ctx.data.project_slug.clone(),
        },
    ];
    if !d.classification.trim().is_empty() {
        fields.push(KeyValue {
            key: "Classification".into(),
            value: d.classification.clone(),
        });
    }
    fields.retain(|f| !f.value.trim().is_empty());
    vec![
        Block::Cover {
            kicker: ctx.data.project_name.clone(),
            title: ctx.title(),
            subtitle: ctx.def.report_type.description().into(),
            fields,
            signatories: d
                .signatories
                .iter()
                .map(|s| Signatory {
                    role: s.role.clone(),
                    name: s.name.clone(),
                })
                .collect(),
            note: None,
        },
        Block::PageBreak,
    ]
}

fn default_introduction(ctx: &Ctx) -> Vec<Block> {
    let data = ctx.data;
    match ctx.def.report_type {
        ReportType::Vcd => vec![
            Block::para(vec![
                Inline::text(
                    "This Verification Control Document (VCD) records, for each requirement of ",
                ),
                Inline::bold(&data.project_name),
                Inline::text(
                    ", how and when it is verified, the evidence that demonstrates compliance and its close-out status. It is a living document; this issue reports the status on ",
                ),
                Inline::text(ctx.generated_on),
                Inline::text("."),
            ]),
            Block::para(vec![
                Inline::text("It was generated by Marreq from project "),
                Inline::code(&data.project_slug),
                Inline::text(format!(
                    ": {} requirements, {} verifications and {} traceability links.",
                    data.requirements.len(),
                    data.verifications.len(),
                    data.links.len()
                )),
            ]),
        ],
        ReportType::Coverage => vec![Block::para(vec![
            Inline::text(
                "This report summarises the traceability between the requirements and verifications of ",
            ),
            Inline::bold(&data.project_name),
            Inline::text(format!(
                " on {}: coverage, gaps, suspect links and data quality ({} requirements, {} verifications, {} links).",
                ctx.generated_on,
                data.requirements.len(),
                data.verifications.len(),
                data.links.len()
            )),
        ])],
    }
}

fn legend() -> Block {
    Block::Legend {
        items: vec![
            (Tone::Ok, "Compliant".into()),
            (Tone::Warn, "Partially compliant".into()),
            (Tone::Bad, "Non-compliant".into()),
            (Tone::None, "Not assessed".into()),
        ],
    }
}

fn vcd_summary(ctx: &mut Ctx, group_by: &str) -> Vec<Block> {
    let data = ctx.data;
    let mut all = Buckets::default();
    data.requirements.iter().for_each(|r| all.add(r));
    let mut out = vec![
        ctx.heading(1, "Verification summary status"),
        Block::Kpis {
            items: vec![
                Kpi {
                    value: all.total.to_string(),
                    label: "requirements".into(),
                    tone: Tone::Open,
                },
                Kpi {
                    value: pct(all.covered, all.total),
                    label: "with a verification".into(),
                    tone: Tone::Open,
                },
                Kpi {
                    value: pct(all.closed, all.total),
                    label: format!("closed ({})", all.closed),
                    tone: Tone::Ok,
                },
                Kpi {
                    value: all.c.to_string(),
                    label: "compliant".into(),
                    tone: Tone::Ok,
                },
                Kpi {
                    value: all.pc.to_string(),
                    label: "partially compliant".into(),
                    tone: Tone::Warn,
                },
                Kpi {
                    value: all.nc.to_string(),
                    label: "non-compliant".into(),
                    tone: Tone::Bad,
                },
            ],
        },
        legend(),
    ];

    let by = if group_by == "category" {
        "category"
    } else {
        "code prefix"
    };
    out.push(ctx.heading(2, &format!("By {by}")));
    let mut t = Table::new(vec![
        Column::new(
            if group_by == "category" {
                "Category"
            } else {
                "Prefix"
            },
            0.24,
        ),
        Column::new("Req.", 0.07).right(),
        Column::new("Verified", 0.09).right(),
        Column::new("C", 0.06).right(),
        Column::new("PC", 0.06).right(),
        Column::new("NC", 0.06).right(),
        Column::new("Not assessed", 0.1).right(),
        Column::new("Closed", 0.08).right(),
        Column::new("", 0.24),
    ]);
    let row = |label: Cell, b: &Buckets| {
        vec![
            label,
            Cell::text(b.total.to_string()),
            Cell::text(b.covered.to_string()),
            Cell::text(b.c.to_string()),
            Cell::text(b.pc.to_string()),
            Cell::text(b.nc.to_string()),
            Cell::text(b.unassessed.to_string()),
            Cell::of(vec![Inline::bold(pct(b.closed, b.total))]),
            b.bar(),
        ]
    };
    for (group, rows) in grouped(data, group_by) {
        let mut b = Buckets::default();
        rows.iter().for_each(|r| b.add(r));
        t.row(row(Cell::text(group), &b));
    }
    t.total(row(Cell::text("Total"), &all));
    out.push(Block::Table(t));

    out.push(ctx.heading(2, "By verification method"));
    let mut by_method: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for r in &data.requirements {
        let keys = if r.methods.is_empty() {
            vec!["–".to_string()]
        } else {
            r.methods.clone()
        };
        for m in keys {
            let e = by_method.entry(m).or_default();
            e.0 += 1;
            if r.is_closed() {
                e.1 += 1;
            }
        }
    }
    let names: BTreeMap<&str, &str> = data
        .method_legend
        .iter()
        .map(|(l, t)| (l.as_str(), t.as_str()))
        .collect();
    let mut t = Table::new(vec![
        Column::new("Method", 0.4),
        Column::new("Requirements", 0.2).right(),
        Column::new("Closed", 0.2).right(),
        Column::new("Closed %", 0.2).right(),
    ]);
    for (m, (n, closed)) in by_method {
        let label = match names.get(m.as_str()) {
            Some(title) => vec![Inline::bold(&m), Inline::text(format!(" {title}"))],
            None => vec![Inline::text(if m == "–" {
                "No method".to_string()
            } else {
                m.clone()
            })],
        };
        t.row(vec![
            Cell::of(label),
            Cell::text(n.to_string()),
            Cell::text(closed.to_string()),
            Cell::text(pct(closed, n)),
        ]);
    }
    out.push(Block::Table(t));
    out
}

fn verification_codes(data: &ReportData, r: &ReqRow) -> String {
    r.verifications
        .iter()
        .filter_map(|id| data.verification(*id))
        .map(|v| v.label().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn vcd_matrix(ctx: &mut Ctx, options: &Map<String, Value>) -> Vec<Block> {
    let data = ctx.data;
    let group_by = opt_str(options, "group_by", "code_prefix");
    let selected: Vec<String> = options
        .get("columns")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_else(|| MATRIX_COLUMNS.iter().map(|(k, _)| k.to_string()).collect());
    let has = |k: &str| selected.iter().any(|s| s == k);
    // Title and statement share the "Requirement" column.
    let mut cols: Vec<(&str, Column)> = Vec::new();
    for (key, _) in MATRIX_COLUMNS {
        let col = match key {
            "id" => Column::new("Req. ID", 0.1),
            "title" if has("title") || has("text") => Column::new("Requirement", 0.3),
            "parent" => Column::new("Parent", 0.09),
            "method" => Column::new("Method", 0.06).center(),
            "level_stage" => Column::new("Level / stage", 0.09),
            "verifications" => Column::new("Verification", 0.15),
            "evidence" => Column::new("Evidence", 0.09),
            "compliance" => Column::new("Compl.", 0.06).center(),
            "closeout" => Column::new("Close-out", 0.1),
            _ => continue,
        };
        if key == "title" || has(key) {
            cols.push((key, col));
        }
    }
    let mut t = Table::new(cols.iter().map(|(_, c)| c.clone()).collect());
    t.compact = true;
    for (group, rows) in grouped(data, group_by) {
        if group_by != "none" {
            t.group(format!("{group} ({} requirements)", rows.len()));
        }
        for r in rows {
            let vers: Vec<_> = r
                .verifications
                .iter()
                .filter_map(|id| data.verification(*id))
                .collect();
            let mut cells = Vec::new();
            for (key, _) in &cols {
                cells.push(match *key {
                    "id" => Cell::of(vec![Inline::code(r.label())]),
                    "title" => {
                        let mut inl = Vec::new();
                        if has("title") {
                            inl.push(Inline::bold(&r.title));
                        }
                        if has("text") && !r.text.trim().is_empty() {
                            if !inl.is_empty() {
                                inl.push(Inline::LineBreak);
                            }
                            inl.push(Inline::muted(r.text.trim()));
                        }
                        Cell::of(inl)
                    }
                    "parent" => Cell::of(vec![Inline::code(
                        r.parent_code.clone().unwrap_or_else(|| "—".into()),
                    )]),
                    "method" => Cell::text(if r.methods.is_empty() {
                        "—".into()
                    } else {
                        r.methods.join(", ")
                    }),
                    "level_stage" => {
                        let mut levels: Vec<&str> =
                            vers.iter().filter_map(|v| v.level.as_deref()).collect();
                        let mut stages: Vec<&str> =
                            vers.iter().filter_map(|v| v.stage.as_deref()).collect();
                        levels.sort();
                        levels.dedup();
                        stages.sort();
                        stages.dedup();
                        let mut inl = vec![Inline::text(if levels.is_empty() {
                            "—".into()
                        } else {
                            levels.join(", ")
                        })];
                        if !stages.is_empty() {
                            inl.push(Inline::LineBreak);
                            inl.push(Inline::muted(stages.join(", ")));
                        }
                        Cell::of(inl)
                    }
                    "verifications" => {
                        if vers.is_empty() {
                            Cell::of(vec![Inline::muted("none")])
                        } else {
                            let mut inl = Vec::new();
                            for (i, v) in vers.iter().enumerate() {
                                if i > 0 {
                                    inl.push(Inline::LineBreak);
                                }
                                inl.push(Inline::bold(v.label()));
                                inl.push(Inline::text(" "));
                                inl.push(Inline::badge(outcome_tone(v.outcome), &v.status));
                            }
                            Cell::of(inl)
                        }
                    }
                    "evidence" => {
                        let ev: Vec<&str> =
                            vers.iter().filter_map(|v| v.evidence.as_deref()).collect();
                        if ev.is_empty() {
                            Cell::text("—")
                        } else {
                            let mut inl = Vec::new();
                            for (i, e) in ev.iter().enumerate() {
                                if i > 0 {
                                    inl.push(Inline::LineBreak);
                                }
                                inl.push(Inline::code(*e));
                            }
                            Cell::of(inl)
                        }
                    }
                    "compliance" => Cell::of(vec![compliance_badge(r)]),
                    "closeout" => {
                        let mut inl = vec![closeout_badge(r)];
                        if !r.is_closed() || r.compliance.as_deref() == Some("PC") {
                            inl.push(Inline::LineBreak);
                            inl.push(Inline::muted(&r.close_out.reason));
                        }
                        Cell::of(inl)
                    }
                    _ => Cell::text(""),
                });
            }
            t.row(cells);
        }
    }
    let heading = ctx.heading(1, "Verification control matrix");
    vec![Block::Landscape {
        blocks: vec![
            heading,
            Block::text(
                "One row per requirement. Method from the requirement; level, stage and evidence from its verifications; compliance from the reviewer's assessment.",
            ),
            Block::Table(t),
        ],
    }]
}

fn coverage_summary(ctx: &mut Ctx) -> Vec<Block> {
    let data = ctx.data;
    let total = data.requirements.len();
    let covered = data
        .requirements
        .iter()
        .filter(|r| !r.verifications.is_empty())
        .count();
    let suspect = data.links.iter().filter(|l| l.suspect).count();
    let avg = if total == 0 {
        0.0
    } else {
        data.links.len() as f64 / total as f64
    };
    let mut out = vec![
        ctx.heading(1, "Coverage summary"),
        Block::Kpis {
            items: vec![
                Kpi {
                    value: total.to_string(),
                    label: "requirements".into(),
                    tone: Tone::Open,
                },
                Kpi {
                    value: data.verifications.len().to_string(),
                    label: "verifications".into(),
                    tone: Tone::Open,
                },
                Kpi {
                    value: pct(covered, total),
                    label: format!("covered ({covered})"),
                    tone: if covered == total {
                        Tone::Ok
                    } else {
                        Tone::Warn
                    },
                },
                Kpi {
                    value: format!("{avg:.1}"),
                    label: "verifications per requirement".into(),
                    tone: Tone::Open,
                },
                Kpi {
                    value: suspect.to_string(),
                    label: "suspect links".into(),
                    tone: if suspect == 0 { Tone::Ok } else { Tone::Warn },
                },
            ],
        },
    ];
    let count_table = |title: &str, order: &[String], values: Vec<&str>| {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for v in values {
            *counts.entry(v.to_string()).or_default() += 1;
        }
        let mut t = Table::new(vec![
            Column::new(title, 0.5),
            Column::new("Count", 0.25).right(),
            Column::new("Share", 0.25).right(),
        ]);
        let n: usize = counts.values().sum();
        let mut keys: Vec<String> = order
            .iter()
            .filter(|k| counts.contains_key(*k))
            .cloned()
            .collect();
        keys.extend(counts.keys().filter(|k| !order.contains(k)).cloned());
        for k in keys {
            let c = counts[&k];
            t.row(vec![
                Cell::text(&k),
                Cell::text(c.to_string()),
                Cell::text(pct(c, n)),
            ]);
        }
        Block::Table(t)
    };
    out.push(ctx.heading(2, "Requirements by status"));
    out.push(count_table(
        "Status",
        &data.requirement_statuses,
        data.requirements
            .iter()
            .map(|r| r.status.as_str())
            .collect(),
    ));
    out.push(ctx.heading(2, "Verifications by status"));
    out.push(count_table(
        "Status",
        &data.verification_statuses,
        data.verifications
            .iter()
            .map(|v| v.status.as_str())
            .collect(),
    ));
    out
}

fn data_quality(ctx: &mut Ctx) -> Vec<Block> {
    let data = ctx.data;
    let checks: Vec<(&str, Vec<String>)> = vec![
        (
            "Requirements without a reference code",
            data.requirements
                .iter()
                .filter(|r| r.code.trim().is_empty())
                .map(|r| r.title.clone())
                .collect(),
        ),
        (
            "Requirements with an empty statement",
            data.requirements
                .iter()
                .filter(|r| r.text.trim().is_empty())
                .map(|r| r.label().to_string())
                .collect(),
        ),
        (
            "Requirements with an unknown category",
            data.requirements
                .iter()
                .filter(|r| !r.category_known)
                .map(|r| r.label().to_string())
                .collect(),
        ),
        (
            "Requirements without a verification method",
            data.requirements
                .iter()
                .filter(|r| r.methods.is_empty())
                .map(|r| r.label().to_string())
                .collect(),
        ),
        (
            "Verifications without a reference code",
            data.verifications
                .iter()
                .filter(|v| v.code.trim().is_empty())
                .map(|v| v.name.clone())
                .collect(),
        ),
        (
            "Verifications with a missing parent",
            data.verifications
                .iter()
                .filter(|v| !v.parent_known)
                .map(|v| v.label().to_string())
                .collect(),
        ),
    ];
    let mut t = Table::new(vec![
        Column::new("Check", 0.42),
        Column::new("Count", 0.1).right(),
        Column::new("Items", 0.48),
    ]);
    for (check, items) in checks {
        let shown: Vec<_> = items.iter().take(15).cloned().collect();
        let mut list = shown.join(", ");
        if items.len() > shown.len() {
            list.push_str(&format!(", … ({} more)", items.len() - shown.len()));
        }
        t.row(vec![
            Cell::text(check),
            Cell::of(vec![Inline::badge(
                if items.is_empty() {
                    Tone::Ok
                } else {
                    Tone::Warn
                },
                items.len().to_string(),
            )]),
            Cell::of(vec![Inline::muted(list)]),
        ]);
    }
    vec![ctx.heading(1, "Data quality"), Block::Table(t)]
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::reports::data::{LinkRow, VerRow};
    use crate::reports::definition::{DefaultContext, SectionConfig};
    use crate::reports::model::headings;
    use crate::services::verification_control_service::CloseOut;

    fn req(
        id: i32,
        code: &str,
        verifications: Vec<i32>,
        compliance: Option<&str>,
        closed: bool,
    ) -> ReqRow {
        ReqRow {
            id,
            code: code.into(),
            title: format!("{code} title"),
            text: format!("{code} shall work."),
            category: if id % 2 == 0 { "Power" } else { "Thermal" }.into(),
            status: "Draft".into(),
            approval_state: "draft".into(),
            author: "Alice".into(),
            reviewer: "Rita".into(),
            parent_code: None,
            methods: vec!["T".into()],
            verifications,
            compliance: compliance.map(String::from),
            compliance_note: None,
            close_out: CloseOut {
                status: if closed {
                    CloseOutStatus::Closed
                } else {
                    CloseOutStatus::Open
                },
                reason: if closed {
                    "Accepted (C)"
                } else {
                    "VER-2 not run"
                }
                .into(),
            },
            category_known: true,
        }
    }

    fn ver(id: i32, code: &str, outcome: VerificationOutcome, reqs: Vec<i32>) -> VerRow {
        VerRow {
            id,
            code: code.into(),
            name: format!("{code} name"),
            status: format!("{outcome:?}"),
            outcome,
            method: Some("Test".into()),
            level: Some("Subsystem".into()),
            stage: Some("QUAL".into()),
            evidence: Some(format!("TR-{id}")),
            requirement_ids: reqs,
            parent_known: true,
        }
    }

    pub(crate) fn sample() -> ReportData {
        ReportData {
            project_name: "Satellite Demo".into(),
            project_slug: "satellite-demo".into(),
            requirements: vec![
                req(1, "SAT-PWR-001", vec![10], Some("C"), true),
                req(2, "SAT-PWR-002", vec![11], None, false),
                req(3, "SAT-THM-001", vec![], None, false),
            ],
            verifications: vec![
                ver(10, "VER-1", VerificationOutcome::Passed, vec![1]),
                ver(11, "VER-2", VerificationOutcome::NotRun, vec![2]),
                ver(12, "VER-3", VerificationOutcome::Failed, vec![]),
            ],
            links: vec![
                LinkRow {
                    req_id: 1,
                    ver_id: 10,
                    suspect: false,
                    suspect_reason: None,
                },
                LinkRow {
                    req_id: 2,
                    ver_id: 11,
                    suspect: true,
                    suspect_reason: Some("Requirement updated".into()),
                },
            ],
            requirement_statuses: vec!["Draft".into()],
            verification_statuses: vec!["Passed".into(), "NotRun".into(), "Failed".into()],
            method_legend: vec![("T".into(), "Test".into())],
        }
    }

    pub(crate) fn default_def(t: ReportType) -> ReportDefinition {
        ReportDefinition::default_for(
            t,
            &DefaultContext {
                project_name: "Satellite Demo".into(),
                project_slug: "satellite-demo".into(),
                user_name: "Alice".into(),
                today: "2026-10-03".into(),
            },
        )
    }

    fn top_headings(blocks: &[Block]) -> Vec<String> {
        headings(blocks)
            .into_iter()
            .filter(|(level, ..)| *level == 1)
            .map(|(_, number, text, _)| {
                if number.is_empty() {
                    text
                } else {
                    format!("{number} {text}")
                }
            })
            .collect()
    }

    #[test]
    fn every_catalog_section_renders_for_its_types() {
        for t in ReportType::ALL {
            let mut def = default_def(t);
            def.sections.iter_mut().for_each(|s| s.enabled = true);
            let blocks = render(&sample(), &def, "2026-10-03");
            for s in catalog()
                .into_iter()
                .filter(|s| s.report_types.contains(&t))
            {
                let mut only = def.clone();
                only.sections = vec![SectionConfig {
                    key: s.key.into(),
                    enabled: true,
                    options: Map::new(),
                }];
                assert!(
                    !render(&sample(), &only, "2026-10-03").is_empty(),
                    "{} renders nothing",
                    s.key
                );
            }
            assert!(blocks.len() > 10);
        }
    }

    #[test]
    fn sections_follow_the_template_order_and_disabled_ones_are_left_out() {
        let mut def = default_def(ReportType::Vcd);
        assert_eq!(
            top_headings(&render(&sample(), &def, "2026-10-03")),
            [
                "Change record",
                "1 Introduction",
                "2 Applicable and reference documents",
                "3 Definitions and abbreviations",
                "4 Verification approach",
                "5 Verification summary status",
                "6 Verification control matrix",
                "7 Open verification items",
                "8 Traceability checks"
            ]
        );

        def.sections.reverse();
        for s in def.sections.iter_mut() {
            if ["definitions", "approach", "change_record"].contains(&s.key.as_str()) {
                s.enabled = false;
            }
        }
        assert_eq!(
            top_headings(&render(&sample(), &def, "2026-10-03")),
            [
                "1 Traceability checks",
                "2 Open verification items",
                "3 Verification control matrix",
                "4 Verification summary status",
                "5 Applicable and reference documents",
                "6 Introduction"
            ]
        );
    }

    #[test]
    fn matrix_columns_and_grouping_follow_the_options() {
        let mut def = default_def(ReportType::Vcd);
        def.sections = vec![SectionConfig {
            key: "matrix".into(),
            enabled: true,
            options: serde_json::from_value(
                json!({"group_by": "category", "columns": ["id", "compliance", "closeout"]}),
            )
            .unwrap(),
        }];
        let blocks = render(&sample(), &def, "2026-10-03");
        let Block::Landscape { blocks } = &blocks[0] else {
            panic!("matrix is landscape")
        };
        let Some(Block::Table(t)) = blocks.iter().find(|b| matches!(b, Block::Table(_))) else {
            panic!("matrix table")
        };
        let titles: Vec<_> = t.columns.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(titles, ["Req. ID", "Compl.", "Close-out"]);
        let groups: Vec<_> = t
            .rows
            .iter()
            .filter(|r| r.kind == super::super::model::RowKind::Group)
            .map(|r| match &r.cells[0] {
                Cell::Content { inlines } => inlines[0].plain().to_string(),
                _ => String::new(),
            })
            .collect();
        assert_eq!(
            groups,
            ["Power (1 requirements)", "Thermal (2 requirements)"]
        );
    }

    #[test]
    fn custom_introduction_text_replaces_the_standard_one() {
        let mut def = default_def(ReportType::Coverage);
        def.sections = vec![SectionConfig {
            key: "introduction".into(),
            enabled: true,
            options: serde_json::from_value(json!({"text": "First para.\n\nSecond para."}))
                .unwrap(),
        }];
        let blocks = render(&sample(), &def, "2026-10-03");
        let paras: Vec<_> = blocks
            .iter()
            .filter_map(|b| match b {
                Block::Paragraph { inlines } => Some(inlines[0].plain().to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(paras, ["First para.", "Second para."]);
    }
}
