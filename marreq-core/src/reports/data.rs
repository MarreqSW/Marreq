// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Everything a report needs about one project, loaded once.

use std::collections::HashMap;

use crate::app::{AppState, DieselCachedRepo};
use crate::repository::errors::RepoError;
use crate::repository::{
    LookupRepository, MatrixRepository, ProjectsRepository, RequirementsRepository, UserRepository,
    VerificationControlRepository, VerificationsRepository,
};
use crate::services::RequirementService;
use crate::services::verification_control_service::{
    CloseOut, CloseOutStatus, VerificationControlService,
};
use crate::status_enums::VerificationOutcome;

#[derive(Debug, Clone)]
pub struct ReqRow {
    pub id: i32,
    pub code: String,
    pub title: String,
    pub text: String,
    pub category: String,
    pub status: String,
    pub approval_state: String,
    pub author: String,
    pub reviewer: String,
    pub parent_code: Option<String>,
    /// VCD method letters (T, A, R, I…), one per verification method.
    pub methods: Vec<String>,
    /// Linked verification ids, in reference-code order.
    pub verifications: Vec<i32>,
    pub compliance: Option<String>,
    pub compliance_note: Option<String>,
    pub close_out: CloseOut,
    pub category_known: bool,
}

impl ReqRow {
    pub fn label(&self) -> &str {
        if self.code.trim().is_empty() {
            &self.title
        } else {
            &self.code
        }
    }

    pub fn is_closed(&self) -> bool {
        self.close_out.status == CloseOutStatus::Closed
    }
}

#[derive(Debug, Clone)]
pub struct VerRow {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub status: String,
    pub outcome: VerificationOutcome,
    pub method: Option<String>,
    pub level: Option<String>,
    pub stage: Option<String>,
    pub evidence: Option<String>,
    pub requirement_ids: Vec<i32>,
    pub parent_known: bool,
}

impl VerRow {
    pub fn label(&self) -> &str {
        if self.code.trim().is_empty() {
            &self.name
        } else {
            &self.code
        }
    }
}

#[derive(Debug, Clone)]
pub struct LinkRow {
    pub req_id: i32,
    pub ver_id: i32,
    pub suspect: bool,
    pub suspect_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ReportData {
    pub project_name: String,
    pub project_slug: String,
    /// Sorted by reference code.
    pub requirements: Vec<ReqRow>,
    /// Sorted by reference code.
    pub verifications: Vec<VerRow>,
    pub links: Vec<LinkRow>,
    /// Requirement status titles in catalog order (for stable tables).
    pub requirement_statuses: Vec<String>,
    pub verification_statuses: Vec<String>,
    /// Method letter → title (e.g. "T" → "Test"), for the definitions table.
    pub method_legend: Vec<(String, String)>,
}

impl ReportData {
    pub fn requirement(&self, id: i32) -> Option<&ReqRow> {
        self.requirements.iter().find(|r| r.id == id)
    }

    pub fn verification(&self, id: i32) -> Option<&VerRow> {
        self.verifications.iter().find(|v| v.id == id)
    }
}

/// VCD method letter for a verification method (Test → T, Analysis → A,
/// Review of design → R, Inspection → I, Demonstration → D).
pub fn method_letter(title: &str, tag: &str) -> String {
    let t = format!("{} {}", title, tag).to_lowercase();
    for (needle, letter) in [
        ("test", "T"),
        ("analys", "A"),
        ("review", "R"),
        ("design", "R"),
        ("inspect", "I"),
        ("demonstr", "D"),
        ("similar", "S"),
    ] {
        if t.contains(needle) {
            return letter.to_string();
        }
    }
    tag.chars()
        .next()
        .or_else(|| title.chars().next())
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".into())
}

/// "SAT-AOCS-001" → "SAT-AOCS": the code without its last segment.
pub fn code_prefix(code: &str) -> String {
    let code = code.trim();
    if code.is_empty() {
        return "(no code)".into();
    }
    match code.rsplit_once(['-', '_', '.']) {
        Some((prefix, _)) if !prefix.is_empty() => prefix.to_string(),
        _ => "(no prefix)".into(),
    }
}

fn by_code<T>(items: &mut [T], code: impl Fn(&T) -> &str) {
    items.sort_by(|a, b| {
        let (a, b) = (code(a), code(b));
        // Empty codes last, then natural order of the code.
        a.is_empty()
            .cmp(&b.is_empty())
            .then_with(|| natural_key(a).cmp(&natural_key(b)))
    });
}

/// Splits digits from text so "REQ-10" sorts after "REQ-9".
fn natural_key(s: &str) -> Vec<(String, u64)> {
    let mut out = Vec::new();
    let mut text = String::new();
    let mut digits = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
        } else {
            if !digits.is_empty() {
                out.push((
                    std::mem::take(&mut text),
                    digits.parse().unwrap_or(u64::MAX),
                ));
                digits.clear();
            }
            text.push(c);
        }
    }
    out.push((text, digits.parse().unwrap_or(0)));
    out
}

pub fn load(state: &AppState<DieselCachedRepo>, project_id: i32) -> Result<ReportData, RepoError> {
    let mut close_outs = VerificationControlService::new(state).project_close_out(project_id)?;
    // Parents live in requirement version links (as on the requirement page).
    // Resolved before taking the read guard below: the service takes its own.
    let parent_of: HashMap<i32, i32> = {
        let requirements = state.repo_read().get_requirements_by_project(project_id)?;
        let service = RequirementService::new(state);
        requirements
            .iter()
            .filter_map(|r| {
                let from_links = r.current_version_id.and_then(|vid| {
                    service
                        .get_parent_requirement_ids_for_version(vid)
                        .first()
                        .copied()
                });
                from_links.or(r.parent_id).map(|p| (r.id, p))
            })
            .collect()
    };
    let repo = state.repo_read();
    let project = repo.get_project_by_id(project_id)?;
    let users: HashMap<i32, String> = repo
        .get_users_all()?
        .into_iter()
        .map(|u| {
            let label = if u.name.trim().is_empty() {
                u.username
            } else {
                u.name
            };
            (u.id, label)
        })
        .collect();
    let user = |id: i32| users.get(&id).cloned().unwrap_or_else(|| format!("#{id}"));
    let categories: HashMap<i32, String> = repo
        .get_categories_by_project(project_id)?
        .into_iter()
        .map(|c| (c.id, c.title))
        .collect();
    let req_statuses = repo.get_requirement_status_by_project(project_id)?;
    let req_status: HashMap<i32, String> = req_statuses
        .iter()
        .map(|s| (s.id, s.title.clone()))
        .collect();
    let ver_statuses = repo.get_verification_status_by_project(project_id)?;
    let ver_status: HashMap<i32, (String, VerificationOutcome)> = ver_statuses
        .iter()
        .map(|s| {
            (
                s.id,
                (
                    s.title.clone(),
                    VerificationOutcome::parse(&s.outcome).unwrap_or(VerificationOutcome::NotRun),
                ),
            )
        })
        .collect();
    let methods: HashMap<i32, (String, String)> = repo
        .get_verification_methods_by_project(project_id)?
        .into_iter()
        .map(|m| (m.id, (method_letter(&m.title, &m.tag), m.title)))
        .collect();
    let mut legend: Vec<(String, String)> = methods.values().cloned().collect();
    legend.sort();
    legend.dedup_by(|a, b| a.0 == b.0);

    let raw_requirements = repo.get_requirements_by_project(project_id)?;
    let code_of: HashMap<i32, String> = raw_requirements
        .iter()
        .map(|r| (r.id, r.reference_code.clone()))
        .collect();
    let links: Vec<LinkRow> = repo
        .get_matrix_by_project(project_id)?
        .into_iter()
        .map(|m| LinkRow {
            req_id: m.req_id,
            ver_id: m.verification_id,
            suspect: m.suspect,
            suspect_reason: m.suspect_reason,
        })
        .collect();

    let mut requirements = Vec::with_capacity(raw_requirements.len());
    for r in raw_requirements {
        let mut method_letters: Vec<String> = repo
            .get_verification_method_ids_for_requirement(r.id)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|id| methods.get(&id).map(|(l, _)| l.clone()))
            .collect();
        method_letters.sort();
        method_letters.dedup();
        let view = close_outs.remove(&r.id);
        let (verifications, compliance, note, close_out) = match view {
            Some(v) => (
                v.verifications.iter().map(|lv| lv.id).collect(),
                v.compliance,
                v.note,
                v.close_out,
            ),
            None => (
                Vec::new(),
                None,
                None,
                CloseOut {
                    status: CloseOutStatus::Open,
                    reason: "No verification linked".into(),
                },
            ),
        };
        requirements.push(ReqRow {
            id: r.id,
            code: r.reference_code.clone(),
            title: r.title.clone(),
            text: r.description.clone(),
            category: categories
                .get(&r.category_id)
                .cloned()
                .unwrap_or_else(|| "(no category)".into()),
            category_known: categories.contains_key(&r.category_id),
            status: req_status
                .get(&r.status_id)
                .cloned()
                .unwrap_or_else(|| format!("Status #{}", r.status_id)),
            approval_state: r.approval_state.clone(),
            author: user(r.author_id),
            reviewer: user(r.reviewer_id),
            parent_code: parent_of.get(&r.id).and_then(|p| code_of.get(p).cloned()),
            methods: method_letters,
            verifications,
            compliance,
            compliance_note: note,
            close_out,
        });
    }
    by_code(&mut requirements, |r| r.code.as_str());

    let control: HashMap<i32, _> = repo
        .list_verification_control_by_project(project_id)?
        .into_iter()
        .map(|c| (c.verification_id, c))
        .collect();
    let raw_verifications = repo.get_verifications_by_project(project_id)?;
    let ver_ids: std::collections::HashSet<i32> = raw_verifications.iter().map(|v| v.id).collect();
    let mut verifications: Vec<VerRow> = raw_verifications
        .into_iter()
        .map(|v| {
            let (status, outcome) = ver_status.get(&v.status_id).cloned().unwrap_or_else(|| {
                (
                    format!("Status #{}", v.status_id),
                    VerificationOutcome::NotRun,
                )
            });
            let c = control.get(&v.id);
            VerRow {
                id: v.id,
                code: v.reference_code.clone(),
                name: v.name.clone(),
                status,
                outcome,
                method: v
                    .verification_method_id
                    .and_then(|m| methods.get(&m).map(|(_, t)| t.clone())),
                level: c.and_then(|c| c.verification_level.clone()),
                stage: c.and_then(|c| c.verification_stage.clone()),
                evidence: c.and_then(|c| c.evidence_reference.clone()),
                requirement_ids: links
                    .iter()
                    .filter(|l| l.ver_id == v.id)
                    .map(|l| l.req_id)
                    .collect(),
                parent_known: v.parent_id.is_none_or(|p| ver_ids.contains(&p)),
            }
        })
        .collect();
    by_code(&mut verifications, |v| v.code.as_str());

    Ok(ReportData {
        project_name: project.name,
        project_slug: project.slug,
        requirements,
        verifications,
        links,
        requirement_statuses: req_statuses.into_iter().map(|s| s.title).collect(),
        verification_statuses: ver_statuses.into_iter().map(|s| s.title).collect(),
        method_legend: legend,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_letters_follow_the_ecss_codes() {
        assert_eq!(method_letter("Test", "TEST"), "T");
        assert_eq!(method_letter("Analysis", "ANALYSIS"), "A");
        assert_eq!(method_letter("Review", "REVIEW"), "R");
        assert_eq!(method_letter("Review of design", "ROD"), "R");
        assert_eq!(method_letter("Inspection", "INSP"), "I");
        assert_eq!(method_letter("Default", "DEF"), "D");
        assert_eq!(method_letter("Simulation", "SIM"), "S");
        assert_eq!(method_letter("Calibration", ""), "C");
    }

    #[test]
    fn code_prefix_drops_the_last_segment() {
        assert_eq!(code_prefix("SAT-AOCS-001"), "SAT-AOCS");
        assert_eq!(code_prefix("REQ_12"), "REQ");
        assert_eq!(code_prefix("1.2.3"), "1.2");
        assert_eq!(code_prefix("REQ1"), "(no prefix)");
        assert_eq!(code_prefix("  "), "(no code)");
    }

    #[test]
    fn codes_sort_naturally_with_empty_codes_last() {
        let mut codes = vec!["REQ-10", "", "REQ-9", "ABC-2", "REQ-1"];
        by_code(&mut codes, |c| c);
        assert_eq!(codes, ["ABC-2", "REQ-1", "REQ-9", "REQ-10", ""]);
    }
}
