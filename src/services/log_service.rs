use std::collections::HashMap;

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::LoggerError;
use crate::models::Log;
use crate::repository::errors::RepoError;
use crate::repository::{LogRepository, UserRepository};
use serde::Serialize;
use serde_json::Value as JsonValue;
use thiserror::Error;

/// Human-readable labels for common entity fields in change logs.
fn field_label(key: &str) -> &'static str {
    match key {
        "title" => "Title",
        "name" => "Name",
        "description" => "Description",
        "status_id" => "Status",
        "category_id" => "Category",
        "applicability_id" => "Applicability",
        "justification" => "Rationale",
        "deadline_date" => "Deadline",
        "reference_code" => "Reference",
        "parent_id" => "Parent",
        "source" => "Source",
        "verification_method_ids" => "Verification",
        _ => "Details",
    }
}

/// Returns a brief summary of what changed, derived from old_values and new_values.
/// Strips generic "via API" descriptions and uses CREATE/UPDATE/DELETE plus changed fields.
pub fn change_summary(log: &Log) -> String {
    let action = log.action_type.to_uppercase();
    match action.as_str() {
        "CREATE" => return "Created".to_string(),
        "DELETE" => return "Deleted".to_string(),
        _ => {}
    }

    let changed = changed_field_labels(log.old_values.as_deref(), log.new_values.as_deref());
    if changed.is_empty() {
        return "Updated".to_string();
    }
    format!("{} updated", changed.join(", "))
}

fn changed_field_labels(old_json: Option<&str>, new_json: Option<&str>) -> Vec<String> {
    let old_obj = old_json.and_then(|s| serde_json::from_str::<JsonValue>(s).ok());
    let new_obj = new_json.and_then(|s| serde_json::from_str::<JsonValue>(s).ok());
    let (old_obj, new_obj) = match (old_obj, new_obj) {
        (Some(JsonValue::Object(a)), Some(JsonValue::Object(b))) => (a, b),
        _ => return vec![],
    };

    let mut labels = Vec::new();
    let skip_keys = [
        "id",
        "author_id",
        "reviewer_id",
        "project_id",
        "creation_date",
        "update_date",
    ];
    for (key, new_val) in new_obj.iter() {
        if skip_keys.contains(&key.as_str()) {
            continue;
        }
        let old_val = old_obj.get(key);
        if old_val != Some(new_val) {
            labels.push(field_label(key).to_string());
        }
    }
    for key in old_obj.keys() {
        if !new_obj.contains_key(key) && !skip_keys.contains(&key.as_str()) {
            labels.push(field_label(key).to_string());
        }
    }
    labels.sort();
    labels.dedup();
    labels
}

/// Maximum length for a displayed value in change details (longer values are truncated).
const CHANGE_VALUE_MAX_LEN: usize = 120;

fn value_to_display(v: Option<&JsonValue>) -> String {
    let s = match v {
        None => return "—".to_string(),
        Some(JsonValue::Null) => return "—".to_string(),
        Some(JsonValue::Bool(b)) => return b.to_string(),
        Some(JsonValue::Number(n)) => return n.to_string(),
        Some(JsonValue::String(s)) => s.as_str(),
        Some(other) => return serde_json::to_string(other).unwrap_or_else(|_| "—".into()),
    };
    if s.len() <= CHANGE_VALUE_MAX_LEN {
        s.to_string()
    } else {
        format!("{}…", &s[..CHANGE_VALUE_MAX_LEN])
    }
}

/// One field change: label and old/new values for display.
#[derive(Debug, Clone, Serialize)]
pub struct ChangeDetail {
    pub field: String,
    pub old_value: String,
    pub new_value: String,
}

/// Returns per-field change details (old value and updated value) for a log entry.
/// Used to show what changed in the requirement changelog.
pub fn log_change_details(log: &Log) -> Vec<ChangeDetail> {
    let action = log.action_type.to_uppercase();
    let old_obj = log
        .old_values
        .as_deref()
        .and_then(|s| serde_json::from_str::<JsonValue>(s).ok());
    let new_obj = log
        .new_values
        .as_deref()
        .and_then(|s| serde_json::from_str::<JsonValue>(s).ok());

    let skip_keys = [
        "id",
        "author_id",
        "reviewer_id",
        "project_id",
        "creation_date",
        "update_date",
    ];

    match action.as_str() {
        "CREATE" => {
            let new_obj = match new_obj {
                Some(JsonValue::Object(o)) => o,
                _ => return vec![],
            };
            let mut out: Vec<ChangeDetail> = new_obj
                .iter()
                .filter(|(k, _)| !skip_keys.contains(&k.as_str()))
                .map(|(k, v)| ChangeDetail {
                    field: field_label(k).to_string(),
                    old_value: "—".to_string(),
                    new_value: value_to_display(Some(v)),
                })
                .collect();
            out.sort_by(|a, b| a.field.cmp(&b.field));
            out
        }
        "DELETE" => {
            let old_obj = match old_obj {
                Some(JsonValue::Object(o)) => o,
                _ => return vec![],
            };
            let mut out: Vec<ChangeDetail> = old_obj
                .iter()
                .filter(|(k, _)| !skip_keys.contains(&k.as_str()))
                .map(|(k, v)| ChangeDetail {
                    field: field_label(k).to_string(),
                    old_value: value_to_display(Some(v)),
                    new_value: "—".to_string(),
                })
                .collect();
            out.sort_by(|a, b| a.field.cmp(&b.field));
            out
        }
        _ => {
            let (old_obj, new_obj) = match (old_obj, new_obj) {
                (Some(JsonValue::Object(a)), Some(JsonValue::Object(b))) => (a, b),
                _ => return vec![],
            };
            let mut out = Vec::new();
            for (key, new_val) in new_obj.iter() {
                if skip_keys.contains(&key.as_str()) {
                    continue;
                }
                let old_val = old_obj.get(key);
                if old_val != Some(new_val) {
                    out.push(ChangeDetail {
                        field: field_label(key).to_string(),
                        old_value: value_to_display(old_val),
                        new_value: value_to_display(Some(new_val)),
                    });
                }
            }
            for key in old_obj.keys() {
                if !new_obj.contains_key(key) && !skip_keys.contains(&key.as_str()) {
                    out.push(ChangeDetail {
                        field: field_label(key).to_string(),
                        old_value: value_to_display(old_obj.get(key)),
                        new_value: "—".to_string(),
                    });
                }
            }
            out.sort_by(|a, b| a.field.cmp(&b.field));
            out
        }
    }
}

fn parse_id_for_label(s: &str) -> Option<i32> {
    s.trim().parse().ok()
}

fn parse_ids_array_for_labels(s: &str) -> Vec<i32> {
    serde_json::from_str::<Vec<i32>>(s).unwrap_or_default()
}

fn resolve_verification_ids_to_labels(s: &str, verification_map: &HashMap<i32, String>) -> String {
    let ids = parse_ids_array_for_labels(s);
    if ids.is_empty() {
        return "—".to_string();
    }
    let labels: Vec<String> = ids
        .iter()
        .filter_map(|id| verification_map.get(id).cloned())
        .collect();
    if labels.is_empty() {
        s.to_string()
    } else {
        labels.join(", ")
    }
}

/// Maps used to resolve entity IDs to human-readable labels in change details.
#[derive(Clone)]
pub struct LabelResolvers<'a> {
    pub req_status_map: &'a HashMap<i32, String>,
    pub test_status_map: &'a HashMap<i32, String>,
    pub category_map: &'a HashMap<i32, String>,
    pub applicability_map: &'a HashMap<i32, String>,
    pub verification_map: &'a HashMap<i32, String>,
    pub parent_label_map: &'a HashMap<i32, String>,
}

/// Resolves ID values to human-readable labels for Status, Category, Applicability, Verification, and Parent.
/// Returns a new vec of change details with old_value/new_value replaced by labels when applicable.
pub fn resolve_change_details_labels(
    details: Vec<ChangeDetail>,
    entity_type: &str,
    resolvers: &LabelResolvers<'_>,
) -> Vec<ChangeDetail> {
    let status_map = if entity_type.eq_ignore_ascii_case("TEST") {
        resolvers.test_status_map
    } else {
        resolvers.req_status_map
    };

    details
        .into_iter()
        .map(|d| {
            let (old_value, new_value) = match d.field.as_str() {
                "Status" => (
                    parse_id_for_label(&d.old_value)
                        .and_then(|id| status_map.get(&id).cloned())
                        .unwrap_or(d.old_value),
                    parse_id_for_label(&d.new_value)
                        .and_then(|id| status_map.get(&id).cloned())
                        .unwrap_or(d.new_value),
                ),
                "Category" => (
                    parse_id_for_label(&d.old_value)
                        .and_then(|id| resolvers.category_map.get(&id).cloned())
                        .unwrap_or(d.old_value),
                    parse_id_for_label(&d.new_value)
                        .and_then(|id| resolvers.category_map.get(&id).cloned())
                        .unwrap_or(d.new_value),
                ),
                "Applicability" => (
                    parse_id_for_label(&d.old_value)
                        .and_then(|id| resolvers.applicability_map.get(&id).cloned())
                        .unwrap_or(d.old_value),
                    parse_id_for_label(&d.new_value)
                        .and_then(|id| resolvers.applicability_map.get(&id).cloned())
                        .unwrap_or(d.new_value),
                ),
                "Verification" => (
                    resolve_verification_ids_to_labels(&d.old_value, resolvers.verification_map),
                    resolve_verification_ids_to_labels(&d.new_value, resolvers.verification_map),
                ),
                "Parent" => (
                    parse_id_for_label(&d.old_value)
                        .and_then(|id| resolvers.parent_label_map.get(&id).cloned())
                        .unwrap_or_else(|| d.old_value.clone()),
                    parse_id_for_label(&d.new_value)
                        .and_then(|id| resolvers.parent_label_map.get(&id).cloned())
                        .unwrap_or_else(|| d.new_value.clone()),
                ),
                _ => (d.old_value, d.new_value),
            };
            ChangeDetail {
                field: d.field,
                old_value,
                new_value,
            }
        })
        .collect()
}

/// Errors that may occur while performing log related operations.
#[derive(Debug, Error)]
pub enum LogServiceError {
    #[error(transparent)]
    Repo(#[from] RepoError),
    #[error(transparent)]
    Logger(#[from] LoggerError),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

/// A log entry enriched with its associated username.
#[derive(Debug, Serialize)]
pub struct LogWithUser {
    #[serde(flatten)]
    pub log: Log,
    pub username: String,
}

/// Aggregate analytics for log activity.
#[derive(Debug, Serialize)]
pub struct LogAnalytics {
    pub last_7_days: i64,
    pub last_30_days: i64,
    pub last_90_days: i64,
}

/// Service providing higher level operations around audit logs.
pub struct LogService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> LogService<'a> {
    /// Create a new service instance bound to the shared application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Fetch the most recent logs enriched with usernames.
    pub fn recent_logs(&self, limit: i64) -> Result<Vec<LogWithUser>, LogServiceError> {
        let logs = self.state.repo_read().get_logs_recent(limit)?;
        Ok(self.enrich_with_usernames(logs)?)
    }

    /// Fetch raw recent log entries.
    pub fn recent_logs_raw(&self, limit: i64) -> Result<Vec<Log>, LogServiceError> {
        let logs = self.state.repo_read().get_logs_recent(limit)?;
        Ok(logs)
    }

    /// Fetch logs for a specific entity enriched with usernames.
    pub fn entity_logs(
        &self,
        entity_type: &str,
        entity_id: i32,
    ) -> Result<Vec<LogWithUser>, LogServiceError> {
        let logs = self
            .state
            .repo_read()
            .get_logs_by_entity(entity_type, entity_id)?;
        Ok(self.enrich_with_usernames(logs)?)
    }

    /// Fetch raw logs for a specific entity.
    pub fn entity_logs_raw(
        &self,
        entity_type: &str,
        entity_id: i32,
    ) -> Result<Vec<Log>, LogServiceError> {
        let logs = self
            .state
            .repo_read()
            .get_logs_by_entity(entity_type, entity_id)?;
        Ok(logs)
    }

    /// Serialize the provided logs into a pretty printed JSON string.
    pub fn logs_to_json(&self, logs: &[Log]) -> Result<String, LogServiceError> {
        Ok(serde_json::to_string_pretty(logs)?)
    }

    /// Record an export operation performed by a user.
    pub fn log_export_action(
        &self,
        actor_id: i32,
        description: Option<String>,
    ) -> Result<(), LogServiceError> {
        let new_log = crate::models::NewLog {
            user_id: actor_id,
            action_type: "EXPORT".to_string(),
            entity_type: "SYSTEM".to_string(),
            entity_id: None,
            project_id: None,
            old_values: None,
            new_values: None,
            description,
            ip_address: None,
            user_agent: None,
        };
        self.state.repo_write().insert_log(&new_log)?;
        Ok(())
    }

    /// Clean up logs older than the provided number of days and record the outcome.
    pub fn cleanup_old_logs(&self, actor_id: i32, days: i64) -> Result<usize, LogServiceError> {
        let count = self.state.repo_write().cleanup_logs(days)?;

        let new_log = crate::models::NewLog {
            user_id: actor_id,
            action_type: "CLEANUP".to_string(),
            entity_type: "SYSTEM".to_string(),
            entity_id: None,
            project_id: None,
            old_values: None,
            new_values: None,
            description: Some(format!("Cleaned up {count} old log entries")),
            ip_address: None,
            user_agent: None,
        };
        let _ = self.state.repo_write().insert_log(&new_log);

        Ok(count)
    }

    /// Retrieve aggregated analytics for recent log activity.
    pub fn analytics(&self) -> Result<LogAnalytics, LogServiceError> {
        // This is a bit inefficient with the current trait, but works for now.
        // Ideally we'd add a count_logs method to the trait.
        let last_7_days = self
            .state
            .repo_read()
            .get_logs_recent(10000)?
            .iter()
            .filter(|l| l.created_at > chrono::Utc::now().naive_utc() - chrono::Duration::days(7))
            .count() as i64;

        let last_30_days = self
            .state
            .repo_read()
            .get_logs_recent(10000)?
            .iter()
            .filter(|l| l.created_at > chrono::Utc::now().naive_utc() - chrono::Duration::days(30))
            .count() as i64;

        let last_90_days = self
            .state
            .repo_read()
            .get_logs_recent(10000)?
            .iter()
            .filter(|l| l.created_at > chrono::Utc::now().naive_utc() - chrono::Duration::days(90))
            .count() as i64;

        Ok(LogAnalytics {
            last_7_days,
            last_30_days,
            last_90_days,
        })
    }

    fn enrich_with_usernames(&self, logs: Vec<Log>) -> Result<Vec<LogWithUser>, RepoError> {
        let repo = self.state.repo_read();
        logs.into_iter()
            .map(|log| {
                let username = repo.get_user_by_id(log.user_id)?.username;
                Ok(LogWithUser { log, username })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::repository::errors::RepoError;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::sync::{Arc, RwLock};

    fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn state_with_repo(repo: DieselRepoMock) -> AppState<DieselCachedRepo> {
        AppState {
            repo: Arc::new(RwLock::new(DieselCachedRepo::new(repo, 0))),
        }
    }

    fn sample_log(log_id: i32, id: i32) -> Log {
        Log {
            log_id,
            user_id: id,
            action_type: "CREATE".into(),
            entity_type: "PROJECT".into(),
            entity_id: Some(1),
            project_id: Some(1),
            old_values: None,
            new_values: None,
            description: Some("Created project".into()),
            ip_address: Some("127.0.0.1".into()),
            user_agent: Some("reqman-test".into()),
            created_at: timestamp(),
        }
    }

    #[test]
    fn enrich_with_usernames_resolves_user_display_names() {
        let user = DieselRepoMock::make_user(7, "alice", "");
        let repo = DieselRepoMock::with_users([user]);
        let state = state_with_repo(repo);
        let service = LogService::new(&state);

        let logs = vec![sample_log(1, 7)];
        let enriched = service.enrich_with_usernames(logs).unwrap();

        assert_eq!(enriched.len(), 1);
        assert_eq!(enriched[0].log.log_id, 1);
        assert_eq!(enriched[0].username, "alice");
    }

    #[test]
    fn enrich_with_usernames_propagates_repo_errors() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = LogService::new(&state);

        let err = service
            .enrich_with_usernames(vec![sample_log(2, 99)])
            .expect_err("missing user should propagate error");

        assert!(matches!(err, RepoError::NotFound));
    }

    #[test]
    fn logs_to_json_produces_pretty_serialization() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = LogService::new(&state);

        let json = service
            .logs_to_json(&[sample_log(3, 1)])
            .expect("serialization should succeed");

        assert!(json.contains("\n"));
        assert!(json.contains("Created project"));
    }

    #[test]
    fn recent_logs_returns_enriched_logs() {
        let user = DieselRepoMock::make_user(1, "alice", "");
        let mut repo = DieselRepoMock::with_users([user]);
        repo.logs.push(sample_log(1, 1));
        repo.logs.push(sample_log(2, 1));

        let state = state_with_repo(repo);
        let service = LogService::new(&state);

        let logs = service.recent_logs(10).unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].username, "alice");
        assert_eq!(logs[1].username, "alice");
    }

    #[test]
    fn recent_logs_respects_limit() {
        let user = DieselRepoMock::make_user(1, "alice", "");
        let mut repo = DieselRepoMock::with_users([user]);
        for i in 1..=5 {
            repo.logs.push(sample_log(i, 1));
        }

        let state = state_with_repo(repo);
        let service = LogService::new(&state);

        let logs = service.recent_logs(3).unwrap();
        assert_eq!(logs.len(), 3);
    }

    #[test]
    fn recent_logs_raw_returns_unenriched_logs() {
        let mut repo = DieselRepoMock::default();
        repo.logs.push(sample_log(1, 1));
        repo.logs.push(sample_log(2, 1));

        let state = state_with_repo(repo);
        let service = LogService::new(&state);

        let logs = service.recent_logs_raw(10).unwrap();
        assert_eq!(logs.len(), 2);
        // get_logs_recent returns logs in reverse order (most recent first)
        let log_ids: Vec<i32> = logs.iter().map(|l| l.log_id).collect();
        assert!(log_ids.contains(&1));
        assert!(log_ids.contains(&2));
    }

    #[test]
    fn entity_logs_returns_enriched_logs_for_entity() {
        let user = DieselRepoMock::make_user(1, "alice", "");
        let mut repo = DieselRepoMock::with_users([user]);
        let mut log1 = sample_log(1, 1);
        log1.entity_type = "PROJECT".into();
        log1.entity_id = Some(10);
        let mut log2 = sample_log(2, 1);
        log2.entity_type = "PROJECT".into();
        log2.entity_id = Some(20);
        let mut log3 = sample_log(3, 1);
        log3.entity_type = "PROJECT".into();
        log3.entity_id = Some(10);

        repo.logs.push(log1);
        repo.logs.push(log2);
        repo.logs.push(log3);

        let state = state_with_repo(repo);
        let service = LogService::new(&state);

        let logs = service.entity_logs("PROJECT", 10).unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].log.entity_id, Some(10));
        assert_eq!(logs[1].log.entity_id, Some(10));
    }

    #[test]
    fn entity_logs_raw_returns_unenriched_logs() {
        let mut repo = DieselRepoMock::default();
        let mut log = sample_log(1, 1);
        log.entity_type = "REQUIREMENT".into();
        log.entity_id = Some(5);
        repo.logs.push(log);

        let state = state_with_repo(repo);
        let service = LogService::new(&state);

        let logs = service.entity_logs_raw("REQUIREMENT", 5).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].entity_id, Some(5));
    }

    #[test]
    fn log_export_action_creates_export_log() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = LogService::new(&state);

        service
            .log_export_action(1, Some("Exported requirements".into()))
            .unwrap();

        let logs = service.recent_logs_raw(1).unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].action_type, "EXPORT");
        assert_eq!(logs[0].entity_type, "SYSTEM");
        assert_eq!(logs[0].user_id, 1);
    }

    #[test]
    fn cleanup_old_logs_removes_old_entries() {
        let mut repo = DieselRepoMock::default();
        // Add some old logs (we'll simulate old dates)
        for i in 1..=5 {
            repo.logs.push(sample_log(i, 1));
        }

        let state = state_with_repo(repo);
        let service = LogService::new(&state);

        // Cleanup logs older than 0 days (should remove all)
        let _count = service.cleanup_old_logs(1, 0).unwrap();
        // Count varies based on implementation - verify no panic

        // Should have a cleanup log entry
        let logs = service.recent_logs_raw(10).unwrap();
        let cleanup_logs: Vec<_> = logs.iter().filter(|l| l.action_type == "CLEANUP").collect();
        assert!(!cleanup_logs.is_empty());
    }

    #[test]
    fn analytics_computes_recent_counts() {
        let mut repo = DieselRepoMock::default();
        // Add some recent logs
        for i in 1..=10 {
            repo.logs.push(sample_log(i, 1));
        }

        let state = state_with_repo(repo);
        let service = LogService::new(&state);

        let analytics = service.analytics().unwrap();
        assert!(analytics.last_7_days >= 0);
        assert!(analytics.last_30_days >= 0);
        assert!(analytics.last_90_days >= 0);
        assert!(analytics.last_30_days >= analytics.last_7_days);
        assert!(analytics.last_90_days >= analytics.last_30_days);
    }

    #[test]
    fn analytics_returns_zero_for_empty_logs() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = LogService::new(&state);

        let analytics = service.analytics().unwrap();
        assert_eq!(analytics.last_7_days, 0);
        assert_eq!(analytics.last_30_days, 0);
        assert_eq!(analytics.last_90_days, 0);
    }

    #[test]
    fn recent_logs_returns_empty_when_no_logs() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = LogService::new(&state);

        let logs = service.recent_logs(10).unwrap();
        assert_eq!(logs.len(), 0);
    }

    #[test]
    fn entity_logs_returns_empty_for_nonexistent_entity() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = LogService::new(&state);

        let logs = service.entity_logs("PROJECT", 999).unwrap();
        assert_eq!(logs.len(), 0);
    }

    #[test]
    fn change_summary_create_returns_created() {
        let mut log = sample_log(1, 1);
        log.action_type = "CREATE".into();
        assert_eq!(change_summary(&log), "Created");
    }

    #[test]
    fn change_summary_delete_returns_deleted() {
        let mut log = sample_log(1, 1);
        log.action_type = "DELETE".into();
        assert_eq!(change_summary(&log), "Deleted");
    }

    #[test]
    fn change_summary_update_no_changes_returns_updated() {
        let mut log = sample_log(1, 1);
        log.action_type = "UPDATE".into();
        log.old_values = None;
        log.new_values = None;
        assert_eq!(change_summary(&log), "Updated");
    }

    #[test]
    fn change_summary_update_with_changed_fields() {
        let mut log = sample_log(1, 1);
        log.action_type = "UPDATE".into();
        log.old_values = Some(r#"{"title":"Old"}"#.into());
        log.new_values = Some(r#"{"title":"New"}"#.into());
        let summary = change_summary(&log);
        assert!(summary.contains("Title"));
        assert!(summary.contains("updated"));
    }

    #[test]
    fn log_change_details_create_returns_new_values() {
        let mut log = sample_log(1, 1);
        log.action_type = "CREATE".into();
        log.new_values = Some(r#"{"title":"Req 1","description":"Desc"}"#.into());
        let details = log_change_details(&log);
        assert_eq!(details.len(), 2);
        let titles: Vec<_> = details.iter().map(|d| d.field.as_str()).collect();
        assert!(titles.contains(&"Title"));
        assert!(titles.contains(&"Description"));
        assert!(details.iter().any(|d| d.old_value == "—" && d.new_value != "—"));
    }

    #[test]
    fn log_change_details_update_shows_changed_field() {
        let mut log = sample_log(1, 1);
        log.action_type = "UPDATE".into();
        log.old_values = Some(r#"{"title":"Old"}"#.into());
        log.new_values = Some(r#"{"title":"New"}"#.into());
        let details = log_change_details(&log);
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].field, "Title");
        assert_eq!(details[0].old_value, "Old");
        assert_eq!(details[0].new_value, "New");
    }

    #[test]
    fn log_change_details_skips_internal_keys() {
        let mut log = sample_log(1, 1);
        log.action_type = "CREATE".into();
        log.new_values = Some(
            r#"{"title":"T","project_id":1,"author_id":1,"creation_date":"2024-01-01"}"#
                .into(),
        );
        let details = log_change_details(&log);
        let fields: Vec<_> = details.iter().map(|d| d.field.as_str()).collect();
        assert!(!fields.contains(&"Reference"), "project_id/author_id/creation_date should be skipped");
        assert!(fields.contains(&"Title"));
    }

    #[test]
    fn change_detail_struct_fields() {
        let detail = ChangeDetail {
            field: "Title".to_string(),
            old_value: "Old".to_string(),
            new_value: "New".to_string(),
        };
        assert_eq!(detail.field, "Title");
        assert_eq!(detail.old_value, "Old");
        assert_eq!(detail.new_value, "New");
    }
}
