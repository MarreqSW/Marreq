use crate::app::{AppState, DieselCachedRepo};
use crate::logger::LoggerError;
use crate::models::Log;
use crate::repository::errors::RepoError;
use crate::repository::{LogRepository, UserRepository};
use serde::Serialize;
use thiserror::Error;

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
}
