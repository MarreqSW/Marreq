use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger, LoggerError};
use crate::models::{ActionType, EntityType, Log};
use crate::repository::errors::RepoError;
use crate::repository::{PooledConnectionWrapper, UserRepository};
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
        let mut conn = self.db_connection()?;
        let logs = Logger::get_recent_logs(conn.as_mut(), limit)?;
        Ok(self.enrich_with_usernames(logs)?)
    }

    /// Fetch raw recent log entries.
    pub fn recent_logs_raw(&self, limit: i64) -> Result<Vec<Log>, LogServiceError> {
        let mut conn = self.db_connection()?;
        let logs = Logger::get_recent_logs(conn.as_mut(), limit)?;
        Ok(logs)
    }

    /// Fetch logs for a specific entity enriched with usernames.
    pub fn entity_logs(
        &self,
        entity_type: &str,
        entity_id: i32,
    ) -> Result<Vec<LogWithUser>, LogServiceError> {
        let mut conn = self.db_connection()?;
        let logs = Logger::get_logs_for_entity(conn.as_mut(), entity_type, entity_id)?;
        Ok(self.enrich_with_usernames(logs)?)
    }

    /// Fetch raw logs for a specific entity.
    pub fn entity_logs_raw(
        &self,
        entity_type: &str,
        entity_id: i32,
    ) -> Result<Vec<Log>, LogServiceError> {
        let mut conn = self.db_connection()?;
        let logs = Logger::get_logs_for_entity(conn.as_mut(), entity_type, entity_id)?;
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
        let mut conn = self.db_connection()?;
        let ctx = LogCtx::new(actor_id);
        Logger::log_export(conn.as_mut(), &ctx, description)?;
        Ok(())
    }

    /// Clean up logs older than the provided number of days and record the outcome.
    pub fn cleanup_old_logs(&self, actor_id: i32, days: i64) -> Result<usize, LogServiceError> {
        let mut conn = self.db_connection()?;
        let ctx = LogCtx::new(actor_id);

        match Logger::cleanup_old_logs(conn.as_mut(), days) {
            Ok(count) => {
                let _ = Logger::log_custom(
                    conn.as_mut(),
                    &ctx,
                    ActionType::StatusChange,
                    EntityType::User,
                    None,
                    None,
                    None,
                    None,
                    Some(format!("Cleaned up {count} old log entries")),
                );
                Ok(count)
            }
            Err(err) => {
                let _ = Logger::log_custom(
                    conn.as_mut(),
                    &ctx,
                    ActionType::StatusChange,
                    EntityType::User,
                    None,
                    None,
                    None,
                    None,
                    Some("Failed to clean up old log entries".to_string()),
                );
                Err(err.into())
            }
        }
    }

    /// Retrieve aggregated analytics for recent log activity.
    pub fn analytics(&self) -> Result<LogAnalytics, LogServiceError> {
        let mut conn = self.db_connection()?;
        let last_7_days = Logger::get_log_count(conn.as_mut(), 7)?;
        let last_30_days = Logger::get_log_count(conn.as_mut(), 30)?;
        let last_90_days = Logger::get_log_count(conn.as_mut(), 90)?;

        Ok(LogAnalytics {
            last_7_days,
            last_30_days,
            last_90_days,
        })
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn enrich_with_usernames(&self, logs: Vec<Log>) -> Result<Vec<LogWithUser>, RepoError> {
        let repo = self.state.repo_read();
        logs.into_iter()
            .map(|log| {
                let username = repo.get_user_by_id(log.user_id)?.user_username;
                Ok(LogWithUser { log, username })
            })
            .collect()
    }
}
