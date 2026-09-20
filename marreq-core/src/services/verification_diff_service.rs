// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Diff two verification snapshots reconstructed from the audit log.

use crate::app::{AppState, DieselCachedRepo};
use crate::diff::{compute_verification_diff, VerificationSnapshotState, VerificationVersionDiff};
use crate::models::{BaselineVerification, EntityType, Log, Verification};
use crate::repository::errors::RepoError;
use crate::repository::{
    BaselineRepository, LogRepository, LookupRepository, VerificationsRepository,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// Snapshot id used when the oldest recorded change is an update (state before that log).
pub const PRE_HISTORY_SNAPSHOT_ID: i32 = 0;

/// Public listing row for the SPA version picker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationSnapshot {
    pub id: i32,
    pub created_at: NaiveDateTime,
    pub name: String,
    pub reference_code: String,
    pub status_id: i32,
}

struct SnapshotRecord {
    id: i32,
    created_at: NaiveDateTime,
    state: VerificationSnapshotState,
}

/// Service for verification snapshot diffs. Read-only; does not modify any data.
pub struct VerificationDiffService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> VerificationDiffService<'a> {
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    fn repo_read(&self) -> std::sync::RwLockReadGuard<'_, DieselCachedRepo> {
        self.state.repo.read().expect("repo lock poisoned")
    }

    /// Chronological snapshots reconstructed from CREATE/UPDATE/STATUS_CHANGE logs.
    pub fn list_snapshots(
        &self,
        project_id: i32,
        verification_id: i32,
    ) -> Result<Vec<VerificationSnapshot>, RepoError> {
        let records = self.load_records(project_id, verification_id)?;
        Ok(records
            .into_iter()
            .map(|record| VerificationSnapshot {
                id: record.id,
                created_at: record.created_at,
                name: record.state.name,
                reference_code: record.state.reference_code,
                status_id: record.state.status_id,
            })
            .collect())
    }

    /// Diff two snapshots (v1 = old, v2 = new). Both ids must belong to this verification.
    pub fn diff_snapshots(
        &self,
        project_id: i32,
        verification_id: i32,
        v1_id: i32,
        v2_id: i32,
    ) -> Result<VerificationVersionDiff, RepoError> {
        let records = self.load_records(project_id, verification_id)?;
        let v1 = records
            .iter()
            .find(|record| record.id == v1_id)
            .ok_or(RepoError::NotFound)?;
        let v2 = records
            .iter()
            .find(|record| record.id == v2_id)
            .ok_or(RepoError::NotFound)?;
        let mut diff = compute_verification_diff(&v1.state, &v2.state);
        self.enrich_diff_with_labels(&mut diff);
        Ok(diff)
    }

    /// Diff a verification as captured in a baseline against its current value.
    pub fn diff_baseline_vs_current(
        &self,
        project_id: i32,
        baseline_id: i32,
        verification_id: i32,
    ) -> Result<VerificationVersionDiff, RepoError> {
        let repo = self.repo_read();
        let baseline = repo.get_baseline_by_id(baseline_id)?;
        if baseline.project_id != project_id {
            return Err(RepoError::NotFound);
        }
        let old = repo
            .get_verifications_for_baseline(baseline_id)?
            .into_iter()
            .find(|snapshot| snapshot.verification_id == verification_id)
            .ok_or(RepoError::NotFound)?;
        let current = repo.get_verification_by_id(verification_id)?;
        if current.project_id != project_id {
            return Err(RepoError::NotFound);
        }
        let mut diff = compute_verification_diff(
            &state_from_baseline_verification(&old),
            &state_from_verification(&current),
        );
        drop(repo);
        self.enrich_diff_with_labels(&mut diff);
        Ok(diff)
    }

    fn load_records(
        &self,
        project_id: i32,
        verification_id: i32,
    ) -> Result<Vec<SnapshotRecord>, RepoError> {
        let repo = self.repo_read();
        let verification: Verification = repo.get_verification_by_id(verification_id)?;
        if verification.project_id != project_id {
            return Err(RepoError::NotFound);
        }
        let logs =
            repo.get_logs_by_entity(&EntityType::Verification.to_string(), verification_id)?;
        Ok(snapshots_from_logs(&logs))
    }

    fn enrich_diff_with_labels(&self, diff: &mut VerificationVersionDiff) {
        let repo = self.repo_read();

        let fill_status = |id: Option<i32>| -> Option<String> {
            id.and_then(|sid| {
                repo.get_verification_status_by_id(sid)
                    .ok()
                    .map(|s| s.title)
            })
        };
        diff.metadata.status.unchanged_label = fill_status(diff.metadata.status.unchanged);
        diff.metadata.status.old_label = fill_status(diff.metadata.status.old_id);
        diff.metadata.status.new_label = fill_status(diff.metadata.status.new_id);

        let fill_method = |id: Option<i32>| -> Option<String> {
            id.and_then(|mid| {
                repo.get_verification_method_by_id(mid)
                    .ok()
                    .map(|m| m.title.clone())
            })
        };
        if diff.metadata.verification_method.old_id.is_none()
            && diff.metadata.verification_method.new_id.is_none()
        {
            diff.metadata.verification_method.unchanged_label = Some("—".into());
        } else {
            diff.metadata.verification_method.unchanged_label =
                fill_method(diff.metadata.verification_method.unchanged);
            diff.metadata.verification_method.old_label =
                fill_method(diff.metadata.verification_method.old_id);
            diff.metadata.verification_method.new_label =
                fill_method(diff.metadata.verification_method.new_id);
        }

        let parent_label = |id: Option<i32>| -> Option<String> {
            id.and_then(|pid| {
                repo.get_verification_by_id(pid).ok().map(|parent| {
                    if parent.reference_code.trim().is_empty() {
                        parent.name
                    } else {
                        format!("{} — {}", parent.reference_code, parent.name)
                    }
                })
            })
        };
        if diff.metadata.parent.old_id.is_none() && diff.metadata.parent.new_id.is_none() {
            diff.metadata.parent.unchanged_label = Some("—".into());
        } else {
            diff.metadata.parent.unchanged_label = parent_label(diff.metadata.parent.unchanged);
            diff.metadata.parent.old_label = parent_label(diff.metadata.parent.old_id);
            diff.metadata.parent.new_label = parent_label(diff.metadata.parent.new_id);
        }
    }
}

fn state_from_verification(value: &Verification) -> VerificationSnapshotState {
    VerificationSnapshotState {
        name: value.name.clone(),
        description: value.description.clone(),
        source: value.source.clone(),
        reference_code: value.reference_code.clone(),
        status_id: value.status_id,
        parent_id: value.parent_id,
        verification_method_id: value.verification_method_id,
    }
}

fn state_from_baseline_verification(value: &BaselineVerification) -> VerificationSnapshotState {
    VerificationSnapshotState {
        name: value.name.clone(),
        description: value.description.clone(),
        source: value.source.clone(),
        reference_code: value.reference_code.clone(),
        status_id: value.status_id,
        parent_id: value.parent_id,
        verification_method_id: value.verification_method_id,
    }
}

fn parse_state(json: &str) -> Option<VerificationSnapshotState> {
    serde_json::from_str(json).ok()
}

fn snapshots_from_logs(logs: &[Log]) -> Vec<SnapshotRecord> {
    let mut ordered: Vec<&Log> = logs.iter().collect();
    ordered.sort_by_key(|log| (log.created_at, log.log_id));

    let mut out = Vec::new();
    for log in ordered {
        let action = log.action_type.to_uppercase();
        if action == "DELETE" {
            continue;
        }
        if out.is_empty() && (action == "UPDATE" || action == "STATUS_CHANGE") {
            if let Some(old) = log.old_values.as_deref().and_then(parse_state) {
                out.push(SnapshotRecord {
                    id: PRE_HISTORY_SNAPSHOT_ID,
                    created_at: log.created_at,
                    state: old,
                });
            }
        }
        if let Some(new) = log.new_values.as_deref().and_then(parse_state) {
            out.push(SnapshotRecord {
                id: log.log_id,
                created_at: log.created_at,
                state: new,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::repository::CacheRepository;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::sync::{Arc, RwLock};

    fn epoch() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2020, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn state_with_repo(repo: DieselRepoMock) -> AppState<DieselCachedRepo> {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    fn verification(id: i32) -> Verification {
        Verification {
            id,
            name: "Power test".into(),
            reference_code: "VER-001".into(),
            description: "Measure 650W".into(),
            source: "TV-001".into(),
            status_id: 1,
            parent_id: None,
            project_id: 1,
            verification_method_id: Some(1),
            author_id: 1,
            reviewer_id: 1,
            status_set_by: None,
            status_set_at: None,
        }
    }

    fn log_row(
        log_id: i32,
        action: &str,
        old_values: Option<&str>,
        new_values: Option<&str>,
        offset_secs: i64,
    ) -> Log {
        Log {
            log_id,
            user_id: 1,
            action_type: action.into(),
            entity_type: "VERIFICATION".into(),
            entity_id: Some(1),
            project_id: Some(1),
            old_values: old_values.map(str::to_string),
            new_values: new_values.map(str::to_string),
            description: None,
            ip_address: None,
            user_agent: None,
            created_at: epoch() + chrono::Duration::seconds(offset_secs),
        }
    }

    const V1: &str = r#"{"name":"Power test","description":"Measure 500W","source":"TV-001","reference_code":"VER-001","status_id":1,"parent_id":null,"verification_method_id":1}"#;
    const V2: &str = r#"{"name":"Power test","description":"Measure 650W","source":"TV-001","reference_code":"VER-001","status_id":2,"parent_id":null,"verification_method_id":1}"#;

    #[test]
    fn snapshots_include_create_and_update() {
        let mut mock = DieselRepoMock::default();
        mock.verifications.insert(1, verification(1));
        mock.logs.push(log_row(10, "CREATE", None, Some(V1), 1));
        mock.logs.push(log_row(11, "UPDATE", Some(V1), Some(V2), 2));
        let state = state_with_repo(mock);
        let service = VerificationDiffService::new(&state);
        let snapshots = service.list_snapshots(1, 1).unwrap();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].id, 10);
        assert_eq!(snapshots[1].id, 11);
        let diff = service.diff_snapshots(1, 1, 10, 11).unwrap();
        assert_eq!(diff.text.description.removed, ["Measure 500W"]);
        assert_eq!(diff.text.description.added, ["Measure 650W"]);
        assert_eq!(diff.metadata.status.old_id, Some(1));
        assert_eq!(diff.metadata.status.new_id, Some(2));
    }

    #[test]
    fn first_update_without_create_exposes_pre_history_snapshot() {
        let mut mock = DieselRepoMock::default();
        mock.verifications.insert(1, verification(1));
        mock.logs.push(log_row(22, "UPDATE", Some(V1), Some(V2), 1));
        let state = state_with_repo(mock);
        let service = VerificationDiffService::new(&state);
        let snapshots = service.list_snapshots(1, 1).unwrap();
        assert_eq!(snapshots[0].id, PRE_HISTORY_SNAPSHOT_ID);
        assert_eq!(snapshots[1].id, 22);
        let diff = service
            .diff_snapshots(1, 1, PRE_HISTORY_SNAPSHOT_ID, 22)
            .unwrap();
        assert_eq!(diff.text.description.removed, ["Measure 500W"]);
    }

    #[test]
    fn wrong_project_is_not_found() {
        let mut mock = DieselRepoMock::default();
        mock.verifications.insert(1, verification(1));
        let state = state_with_repo(mock);
        let service = VerificationDiffService::new(&state);
        assert!(matches!(
            service.list_snapshots(9, 1),
            Err(RepoError::NotFound)
        ));
    }

    #[test]
    fn baseline_snapshot_is_compared_with_current_verification() {
        let mut mock = DieselRepoMock::default();
        mock.baselines.push(crate::models::Baseline {
            id: 7,
            project_id: 1,
            name: "PDR".into(),
            description: None,
            created_at: epoch(),
            created_by: 1,
            source_saved_view_id: None,
            source_view_definition: None,
        });
        mock.baseline_verifications
            .push(crate::models::BaselineVerification {
                baseline_id: 7,
                verification_id: 1,
                name: "Power test".into(),
                reference_code: "VER-001".into(),
                description: "Measure 500W".into(),
                source: "TV-001".into(),
                status_id: 1,
                parent_id: None,
                project_id: 1,
                verification_method_id: Some(1),
                author_id: 1,
                reviewer_id: 1,
            });
        mock.verifications.insert(1, verification(1));
        let state = state_with_repo(mock);
        let diff = VerificationDiffService::new(&state)
            .diff_baseline_vs_current(1, 7, 1)
            .unwrap();
        assert_eq!(diff.text.description.removed, ["Measure 500W"]);
        assert_eq!(diff.text.description.added, ["Measure 650W"]);
    }
}
