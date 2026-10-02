// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Verification control data and requirement close-out (issue #353): the
//! level, stage and evidence of verifications, the reviewer's compliance
//! assessment of requirements, and the derived close-out status the
//! Verification Control Document reports.

use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{
    ActionType, EntityType, NewLog, RequirementCompliance, User, Verification, VerificationControl,
};
use crate::repository::errors::RepoError;
use crate::repository::{
    LogRepository, LookupRepository, MatrixRepository, RequirementsRepository,
    VerificationControlRepository, VerificationsRepository,
};
use crate::status_enums::VerificationOutcome;

/// Longest level or stage label (matches the column size).
pub const LABEL_MAX: usize = 40;
/// Longest evidence reference or compliance note.
pub const TEXT_MAX: usize = 2000;

/// Reviewer's compliance assessment of a requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Compliance {
    /// Compliant.
    C,
    /// Partially compliant, accepted (e.g. with a waiver).
    PC,
    /// Non-compliant.
    NC,
}

impl Compliance {
    pub fn as_str(self) -> &'static str {
        match self {
            Compliance::C => "C",
            Compliance::PC => "PC",
            Compliance::NC => "NC",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "C" => Some(Compliance::C),
            "PC" => Some(Compliance::PC),
            "NC" => Some(Compliance::NC),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseOutStatus {
    Open,
    Closed,
}

/// Whether a requirement's verification is complete, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloseOut {
    pub status: CloseOutStatus,
    pub reason: String,
}

/// One verification linked to a requirement, as close-out sees it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedVerification {
    pub id: i32,
    pub reference_code: String,
    pub name: String,
    pub status_id: i32,
    pub status_title: String,
    pub outcome: VerificationOutcome,
}

impl LinkedVerification {
    fn label(&self) -> &str {
        if self.reference_code.trim().is_empty() {
            &self.name
        } else {
            &self.reference_code
        }
    }
}

/// The close-out rule. A requirement is **closed** when it has at least one
/// linked verification, every linked verification passed, and the reviewer
/// assessed it as compliant (C) or partially compliant (PC). Otherwise it is
/// open, and the reason names what is missing, in this order: no
/// verification, failed, not finished, no assessment, non-compliant.
pub fn close_out(
    verifications: &[LinkedVerification],
    compliance: Option<(Compliance, Option<&str>)>,
) -> CloseOut {
    let open = |reason: String| CloseOut {
        status: CloseOutStatus::Open,
        reason,
    };
    if verifications.is_empty() {
        return open("No verification linked".into());
    }
    let labels = |outcomes: &[VerificationOutcome]| -> Vec<&str> {
        verifications
            .iter()
            .filter(|v| outcomes.contains(&v.outcome))
            .map(LinkedVerification::label)
            .collect()
    };
    let failed = labels(&[VerificationOutcome::Failed]);
    if !failed.is_empty() {
        return open(format!("{} failed", failed.join(", ")));
    }
    let in_progress = labels(&[VerificationOutcome::InProgress]);
    let not_run = labels(&[VerificationOutcome::NotRun]);
    if !in_progress.is_empty() || !not_run.is_empty() {
        let mut parts = Vec::new();
        if !not_run.is_empty() {
            parts.push(format!("{} not run", not_run.join(", ")));
        }
        if !in_progress.is_empty() {
            parts.push(format!("{} in progress", in_progress.join(", ")));
        }
        return open(parts.join("; "));
    }
    match compliance {
        None => open("Compliance not assessed".into()),
        Some((Compliance::NC, note)) => open(match note.map(str::trim).filter(|n| !n.is_empty()) {
            Some(note) => format!("Non-compliant: {note}"),
            None => "Non-compliant".into(),
        }),
        Some((c, _)) => CloseOut {
            status: CloseOutStatus::Closed,
            reason: format!("Accepted ({})", c.as_str()),
        },
    }
}

/// Level, stage and evidence of a verification, with the labels already used
/// in the project (for suggestions in the UI).
#[derive(Debug, Clone, Serialize)]
pub struct VerificationControlView {
    pub verification_id: i32,
    pub verification_level: Option<String>,
    pub verification_stage: Option<String>,
    pub evidence_reference: Option<String>,
    pub updated_by: Option<i32>,
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub suggestions: LabelSuggestions,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct LabelSuggestions {
    pub levels: Vec<String>,
    pub stages: Vec<String>,
}

/// Body of `PUT …/verifications/<id>/control`. Empty strings clear a field.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationControlInput {
    #[serde(default)]
    pub verification_level: Option<String>,
    #[serde(default)]
    pub verification_stage: Option<String>,
    #[serde(default)]
    pub evidence_reference: Option<String>,
}

/// A requirement's compliance assessment and derived close-out.
#[derive(Debug, Clone, Serialize)]
pub struct RequirementCloseOutView {
    pub requirement_id: i32,
    pub compliance: Option<String>,
    pub note: Option<String>,
    pub set_by: Option<i32>,
    pub set_at: Option<chrono::NaiveDateTime>,
    pub close_out: CloseOut,
    pub verifications: Vec<LinkedVerification>,
}

/// Body of `PUT …/requirements/<id>/compliance`. `compliance: null` clears it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComplianceInput {
    pub compliance: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

fn clean(value: Option<String>, max: usize, field: &str) -> Result<Option<String>, RepoError> {
    let value = value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    if let Some(v) = &value
        && v.chars().count() > max
    {
        return Err(RepoError::BadInput(format!(
            "{field} must be at most {max} characters"
        )));
    }
    Ok(value)
}

pub struct VerificationControlService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> VerificationControlService<'a> {
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    fn verification_in_project(
        &self,
        project_id: i32,
        verification_id: i32,
    ) -> Result<Verification, RepoError> {
        let v = self
            .state
            .repo_read()
            .get_verification_by_id(verification_id)?;
        if v.project_id != project_id {
            return Err(RepoError::NotFound);
        }
        Ok(v)
    }

    pub fn suggestions(&self, project_id: i32) -> Result<LabelSuggestions, RepoError> {
        let rows = self
            .state
            .repo_read()
            .list_verification_control_by_project(project_id)?;
        let levels: BTreeSet<String> = rows
            .iter()
            .filter_map(|r| r.verification_level.clone())
            .collect();
        let stages: BTreeSet<String> = rows
            .iter()
            .filter_map(|r| r.verification_stage.clone())
            .collect();
        Ok(LabelSuggestions {
            levels: levels.into_iter().collect(),
            stages: stages.into_iter().collect(),
        })
    }

    pub fn get_verification_control(
        &self,
        project_id: i32,
        verification_id: i32,
    ) -> Result<VerificationControlView, RepoError> {
        self.verification_in_project(project_id, verification_id)?;
        let row = self
            .state
            .repo_read()
            .get_verification_control(verification_id)?;
        Ok(VerificationControlView {
            verification_id,
            verification_level: row.as_ref().and_then(|r| r.verification_level.clone()),
            verification_stage: row.as_ref().and_then(|r| r.verification_stage.clone()),
            evidence_reference: row.as_ref().and_then(|r| r.evidence_reference.clone()),
            updated_by: row.as_ref().and_then(|r| r.updated_by),
            updated_at: row.as_ref().map(|r| r.updated_at),
            suggestions: self.suggestions(project_id)?,
        })
    }

    /// Replace the level, stage and evidence of a verification (audit-logged).
    pub fn set_verification_control(
        &self,
        actor: &User,
        project_id: i32,
        verification_id: i32,
        input: VerificationControlInput,
    ) -> Result<VerificationControlView, RepoError> {
        let verification = self.verification_in_project(project_id, verification_id)?;
        let row = VerificationControl {
            verification_id,
            project_id,
            verification_level: clean(input.verification_level, LABEL_MAX, "verification_level")?,
            verification_stage: clean(input.verification_stage, LABEL_MAX, "verification_stage")?,
            evidence_reference: clean(input.evidence_reference, TEXT_MAX, "evidence_reference")?,
            updated_by: Some(actor.id),
            updated_at: chrono::Utc::now().naive_utc(),
        };
        let before = self
            .state
            .repo_read()
            .get_verification_control(verification_id)?;
        let mut repo = self.state.repo_write();
        repo.upsert_verification_control(&row)?;
        let fields = |c: Option<&VerificationControl>| {
            serde_json::json!({
                "verification_level": c.and_then(|c| c.verification_level.clone()),
                "verification_stage": c.and_then(|c| c.verification_stage.clone()),
                "evidence_reference": c.and_then(|c| c.evidence_reference.clone()),
            })
            .to_string()
        };
        let _ = repo.insert_log(&NewLog {
            user_id: actor.id,
            action_type: ActionType::Update.to_string(),
            entity_type: EntityType::Verification.to_string(),
            entity_id: Some(verification_id),
            project_id: Some(project_id),
            old_values: Some(fields(before.as_ref())),
            new_values: Some(fields(Some(&row))),
            description: Some(format!(
                "Updated verification control data of {}",
                verification_label(&verification)
            )),
            ip_address: None,
            user_agent: None,
        });
        drop(repo);
        self.get_verification_control(project_id, verification_id)
    }

    /// Verifications linked to each requirement of the project, with their
    /// outcome, keyed by requirement id.
    pub fn linked_verifications_by_requirement(
        &self,
        project_id: i32,
    ) -> Result<HashMap<i32, Vec<LinkedVerification>>, RepoError> {
        let repo = self.state.repo_read();
        let statuses: HashMap<i32, (String, VerificationOutcome)> = repo
            .get_verification_status_by_project(project_id)?
            .into_iter()
            .map(|s| {
                let outcome =
                    VerificationOutcome::parse(&s.outcome).unwrap_or(VerificationOutcome::NotRun);
                (s.id, (s.title, outcome))
            })
            .collect();
        let verifications: HashMap<i32, Verification> = repo
            .get_verifications_by_project(project_id)?
            .into_iter()
            .map(|v| (v.id, v))
            .collect();
        let mut by_req: HashMap<i32, Vec<LinkedVerification>> = HashMap::new();
        for link in repo.get_matrix_by_project(project_id)? {
            let Some(v) = verifications.get(&link.verification_id) else {
                continue;
            };
            let (status_title, outcome) =
                statuses.get(&v.status_id).cloned().unwrap_or_else(|| {
                    (
                        format!("Status #{}", v.status_id),
                        VerificationOutcome::NotRun,
                    )
                });
            by_req
                .entry(link.req_id)
                .or_default()
                .push(LinkedVerification {
                    id: v.id,
                    reference_code: v.reference_code.clone(),
                    name: v.name.clone(),
                    status_id: v.status_id,
                    status_title,
                    outcome,
                });
        }
        for list in by_req.values_mut() {
            list.sort_by(|a, b| {
                a.reference_code
                    .cmp(&b.reference_code)
                    .then(a.id.cmp(&b.id))
            });
        }
        Ok(by_req)
    }

    fn close_out_view(
        requirement_id: i32,
        row: Option<RequirementCompliance>,
        verifications: Vec<LinkedVerification>,
    ) -> RequirementCloseOutView {
        let assessment = row
            .as_ref()
            .and_then(|r| Compliance::parse(&r.compliance).map(|c| (c, r.note.as_deref())));
        RequirementCloseOutView {
            requirement_id,
            close_out: close_out(&verifications, assessment),
            compliance: row.as_ref().map(|r| r.compliance.trim().to_string()),
            note: row.as_ref().and_then(|r| r.note.clone()),
            set_by: row.as_ref().and_then(|r| r.set_by),
            set_at: row.as_ref().map(|r| r.set_at),
            verifications,
        }
    }

    /// Close-out of every requirement in the project, keyed by requirement id.
    pub fn project_close_out(
        &self,
        project_id: i32,
    ) -> Result<HashMap<i32, RequirementCloseOutView>, RepoError> {
        let mut linked = self.linked_verifications_by_requirement(project_id)?;
        let repo = self.state.repo_read();
        let mut compliance: HashMap<i32, RequirementCompliance> = repo
            .list_requirement_compliance_by_project(project_id)?
            .into_iter()
            .map(|c| (c.requirement_id, c))
            .collect();
        Ok(repo
            .get_requirements_by_project(project_id)?
            .into_iter()
            .map(|r| {
                let view = Self::close_out_view(
                    r.id,
                    compliance.remove(&r.id),
                    linked.remove(&r.id).unwrap_or_default(),
                );
                (r.id, view)
            })
            .collect())
    }

    pub fn requirement_close_out(
        &self,
        project_id: i32,
        requirement_id: i32,
    ) -> Result<RequirementCloseOutView, RepoError> {
        let requirement = self
            .state
            .repo_read()
            .get_requirement_by_id(requirement_id)?;
        if requirement.project_id != project_id {
            return Err(RepoError::NotFound);
        }
        let verifications = self
            .linked_verifications_by_requirement(project_id)?
            .remove(&requirement_id)
            .unwrap_or_default();
        let row = self
            .state
            .repo_read()
            .get_requirement_compliance(requirement_id)?;
        Ok(Self::close_out_view(requirement_id, row, verifications))
    }

    /// Set or clear the compliance assessment (audit-logged). Callers check
    /// that the actor is a project reviewer.
    pub fn set_requirement_compliance(
        &self,
        actor: &User,
        project_id: i32,
        requirement_id: i32,
        input: ComplianceInput,
    ) -> Result<RequirementCloseOutView, RepoError> {
        let requirement = self
            .state
            .repo_read()
            .get_requirement_by_id(requirement_id)?;
        if requirement.project_id != project_id {
            return Err(RepoError::NotFound);
        }
        let compliance = match input.compliance.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(raw) => Some(Compliance::parse(raw).ok_or_else(|| {
                RepoError::BadInput("compliance must be C, PC, NC or null".into())
            })?),
        };
        let note = clean(input.note, TEXT_MAX, "note")?;
        let before = self
            .state
            .repo_read()
            .get_requirement_compliance(requirement_id)?;
        let mut repo = self.state.repo_write();
        match compliance {
            Some(c) => {
                repo.set_requirement_compliance(&RequirementCompliance {
                    requirement_id,
                    project_id,
                    compliance: c.as_str().to_string(),
                    note: note.clone(),
                    set_by: Some(actor.id),
                    set_at: chrono::Utc::now().naive_utc(),
                })?;
            }
            None => {
                repo.clear_requirement_compliance(requirement_id)?;
            }
        }
        let snapshot = |c: Option<&str>, n: Option<&str>| {
            serde_json::json!({ "compliance": c, "note": n }).to_string()
        };
        let label = if requirement.reference_code.trim().is_empty() {
            format!("requirement #{requirement_id}")
        } else {
            requirement.reference_code.clone()
        };
        let _ = repo.insert_log(&NewLog {
            user_id: actor.id,
            action_type: ActionType::Update.to_string(),
            entity_type: EntityType::Requirement.to_string(),
            entity_id: Some(requirement_id),
            project_id: Some(project_id),
            old_values: Some(snapshot(
                before.as_ref().map(|b| b.compliance.trim()),
                before.as_ref().and_then(|b| b.note.as_deref()),
            )),
            new_values: Some(snapshot(
                compliance.map(Compliance::as_str),
                note.as_deref(),
            )),
            description: Some(match compliance {
                Some(c) => format!("Set compliance of {label} to {}", c.as_str()),
                None => format!("Cleared compliance of {label}"),
            }),
            ip_address: None,
            user_agent: None,
        });
        drop(repo);
        self.requirement_close_out(project_id, requirement_id)
    }
}

fn verification_label(v: &Verification) -> String {
    if v.reference_code.trim().is_empty() {
        format!("verification #{}", v.id)
    } else {
        v.reference_code.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ver(code: &str, outcome: VerificationOutcome) -> LinkedVerification {
        LinkedVerification {
            id: 1,
            reference_code: code.into(),
            name: format!("{code} name"),
            status_id: 1,
            status_title: outcome.as_str().into(),
            outcome,
        }
    }

    fn reason(c: &CloseOut) -> (&CloseOutStatus, &str) {
        (&c.status, c.reason.as_str())
    }

    #[test]
    fn close_out_rules() {
        use CloseOutStatus::*;
        use VerificationOutcome::*;
        let pass = [ver("VER-1", Passed), ver("VER-2", Passed)];

        assert_eq!(
            reason(&close_out(&[], Some((Compliance::C, None)))),
            (&Open, "No verification linked")
        );
        assert_eq!(
            reason(&close_out(
                &[ver("VER-1", Failed), ver("VER-2", NotRun)],
                Some((Compliance::C, None))
            )),
            (&Open, "VER-1 failed")
        );
        assert_eq!(
            reason(&close_out(
                &[
                    ver("VER-1", NotRun),
                    ver("VER-2", InProgress),
                    ver("VER-3", Passed)
                ],
                None
            )),
            (&Open, "VER-1 not run; VER-2 in progress")
        );
        assert_eq!(
            reason(&close_out(&pass, None)),
            (&Open, "Compliance not assessed")
        );
        assert_eq!(
            reason(&close_out(
                &pass,
                Some((Compliance::NC, Some(" NCR-12 open ")))
            )),
            (&Open, "Non-compliant: NCR-12 open")
        );
        assert_eq!(
            reason(&close_out(&pass, Some((Compliance::NC, Some(" "))))),
            (&Open, "Non-compliant")
        );
        assert_eq!(
            reason(&close_out(&pass, Some((Compliance::C, None)))),
            (&Closed, "Accepted (C)")
        );
        assert_eq!(
            reason(&close_out(&pass, Some((Compliance::PC, Some("RFW-3"))))),
            (&Closed, "Accepted (PC)")
        );
    }

    #[test]
    fn close_out_names_a_verification_without_code_by_its_name() {
        let mut v = ver("", VerificationOutcome::Failed);
        v.name = "Thermal vacuum".into();
        assert_eq!(close_out(&[v], None).reason, "Thermal vacuum failed");
    }

    #[test]
    fn compliance_parses_only_the_three_codes() {
        assert_eq!(Compliance::parse(" PC "), Some(Compliance::PC));
        assert_eq!(Compliance::parse("c"), None);
        assert_eq!(Compliance::parse("Y"), None);
    }

    #[test]
    fn clean_trims_empties_and_limits_length() {
        assert_eq!(
            clean(Some("  QUAL ".into()), 40, "stage").unwrap(),
            Some("QUAL".into())
        );
        assert_eq!(clean(Some("   ".into()), 40, "stage").unwrap(), None);
        assert_eq!(clean(None, 40, "stage").unwrap(), None);
        assert!(clean(Some("x".repeat(41)), 40, "stage").is_err());
    }
}
