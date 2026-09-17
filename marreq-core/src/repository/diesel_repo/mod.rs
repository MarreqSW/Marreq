// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::errors::RepoError;
use crate::namespaces::TAKEN_NAMESPACE_MESSAGE;
use diesel::pg::PgConnection;
use diesel::prelude::define_sql_function;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::sql_types::Text;
use diesel::RunQueryDsl;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

define_sql_function!(fn lower(x: Text) -> Text);

/// Map a Diesel DB error to the most specific [`RepoError`] variant:
/// - `UniqueViolation`  → [`RepoError::Duplicate`] with a field-level message.
/// - PL/pgSQL triggers  → [`RepoError::CrossProjectViolation`] when the message
///   begins with the token `[cross_project]`.
/// - Everything else    → [`RepoError::Db`] (the default `From` impl).
pub(crate) fn map_db_error(e: diesel::result::Error) -> RepoError {
    use diesel::result::{DatabaseErrorKind, Error as DE};
    if let DE::DatabaseError(ref kind, ref info) = e {
        match kind {
            DatabaseErrorKind::UniqueViolation => {
                let msg = match info.constraint_name().unwrap_or("") {
                    c if c.contains("username") || c.contains("groups_slug") => {
                        TAKEN_NAMESPACE_MESSAGE.to_string()
                    }
                    c if c.contains("email") => "email is already taken".to_string(),
                    c if c.contains("tests_project_id_reference_code") => {
                        "reference_code is already used in this project".to_string()
                    }
                    c if c.contains("requirement_status_project_id_tag")
                        || c.contains("test_status_project_id_tag")
                        || c.contains("categories_project_id_tag")
                        || c.contains("applicability_project_id_tag")
                        || c.contains("verification_project_id_tag") =>
                    {
                        "tag is already used in this project".to_string()
                    }
                    c if c.contains("projects_slug_unique")
                        || c.contains("idx_projects_owner_slug_unique")
                        || c.contains("idx_projects_group_slug_unique") =>
                    {
                        "project slug is already used".to_string()
                    }
                    _ => "value is already taken".to_string(),
                };
                return RepoError::Duplicate(msg);
            }
            DatabaseErrorKind::Unknown => {
                let msg = info.message();
                if let Some(detail) = msg.strip_prefix("[cross_project]") {
                    return RepoError::CrossProjectViolation(detail.trim().to_string());
                }
            }
            _ => {}
        }
    }
    e.into()
}

/// Compatibility alias kept so existing call-sites in identity-constraint code
/// continue to compile without changes.
#[inline]
pub(crate) fn map_unique_violation(e: diesel::result::Error) -> RepoError {
    map_db_error(e)
}

/// Database connection wrapper for use in Rocket handlers
pub type DbConn = rocket_sync_db_pools::diesel::PgConnection;

/// Connection pool type
pub type ConnectionPool = Pool<ConnectionManager<PgConnection>>;
pub type PooledConn = PooledConnection<ConnectionManager<PgConnection>>;

/// Wrapper for pooled connections that can be used in place of regular connections
pub struct PooledConnectionWrapper {
    inner: PooledConn,
}

impl PooledConnectionWrapper {
    /// Create a new pooled connection wrapper
    pub fn new(pooled_conn: PooledConn) -> Self {
        Self { inner: pooled_conn }
    }

    /// Get a mutable reference to the inner connection
    #[allow(clippy::should_implement_trait)]
    pub fn as_mut(&mut self) -> &mut PgConnection {
        &mut self.inner
    }

    /// Get a reference to the inner connection
    #[allow(clippy::should_implement_trait)]
    pub fn as_ref(&self) -> &PgConnection {
        &self.inner
    }
}

impl std::ops::Deref for PooledConnectionWrapper {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for PooledConnectionWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

static CONNECTION_POOL: OnceLock<Arc<ConnectionPool>> = OnceLock::new();

/// Create the database connection pool. Returns an error if DATABASE_URL is unset or the pool cannot be built.
/// Call this from `app::build_with()` before creating the repository; the pool is stored globally for `DieselRepo::new()`.
pub fn create_connection_pool() -> Result<Arc<ConnectionPool>, Box<dyn std::error::Error>> {
    let database_url = match crate::config::AppConfig::try_current() {
        Some(cfg) => cfg.database_url.clone(),
        None => {
            dotenvy::dotenv().ok();
            std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set")?
        }
    };
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .max_size(30)
        .min_idle(Some(10))
        .connection_timeout(Duration::from_secs(30))
        .idle_timeout(Some(Duration::from_secs(600)))
        .max_lifetime(Some(Duration::from_secs(1800)))
        .build(manager)
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;
    Ok(Arc::new(pool))
}

/// Initialize the global connection pool. Must be called from `app::build_with()` before any `DieselRepo::new()`.
pub fn init_connection_pool() -> Result<(), Box<dyn std::error::Error>> {
    CONNECTION_POOL
        .set(create_connection_pool()?)
        .map_err(|_| "connection pool already initialized".into())
}

fn get_pool() -> Result<Arc<ConnectionPool>, Box<dyn std::error::Error>> {
    CONNECTION_POOL.get().cloned().ok_or_else(|| {
        "connection pool not initialized (call init_connection_pool from app::build_with())".into()
    })
}
/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub max_size: u32,
    pub min_idle: u32,
    pub current_size: u32,
    pub available: u32,
}

impl PoolStats {
    /// Get the utilization percentage of the pool
    pub fn utilization_percentage(&self) -> f64 {
        if self.max_size == 0 {
            0.0
        } else {
            (self.current_size as f64 / self.max_size as f64) * 100.0
        }
    }

    /// Check if the pool is healthy
    pub fn is_healthy(&self) -> bool {
        self.available > 0 && self.current_size <= self.max_size
    }

    /// Get the number of active connections
    pub fn active_connections(&self) -> u32 {
        self.current_size - self.available
    }

    /// Get the pool efficiency (available connections vs total)
    pub fn efficiency(&self) -> f64 {
        if self.current_size == 0 {
            0.0
        } else {
            (self.available as f64 / self.current_size as f64) * 100.0
        }
    }
}

/// Detailed pool information
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub stats: PoolStats,
    pub connection_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_lifetime: Option<Duration>,
}

#[derive(Clone)]
pub struct DieselRepo {
    pool: Arc<ConnectionPool>,
}

impl Default for DieselRepo {
    fn default() -> Self {
        Self::new().expect("database connection pool not initialized")
    }
}

impl DieselRepo {
    /// Create a repository using the global connection pool. Fails if the pool has not been
    /// initialized (e.g. by the first call to `new()` or by pre-initialization in `app::build_with()`).
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self { pool: get_pool()? })
    }

    pub fn get_conn(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.pool
            .get()
            .map(PooledConnectionWrapper::new)
            .map_err(|e| RepoError::Pool(e.to_string()))
    }

    pub fn pool_stats(&self) -> PoolStats {
        PoolStats {
            max_size: self.pool.max_size(),
            min_idle: self.pool.min_idle().unwrap_or(0),
            current_size: self.pool.state().connections,
            available: self.pool.state().idle_connections,
        }
    }

    pub fn pool_info(&self) -> PoolInfo {
        PoolInfo {
            stats: self.pool_stats(),
            connection_timeout: self.pool.connection_timeout(),
            idle_timeout: self.pool.idle_timeout(),
            max_lifetime: self.pool.max_lifetime(),
        }
    }

    pub fn test_pool_health(&self) -> Result<bool, RepoError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| RepoError::Pool(e.to_string()))?;
        diesel::sql_query("SELECT 1")
            .execute(&mut conn)
            .map_err(RepoError::from)?;
        Ok(true)
    }
}

pub(crate) mod baselines;
pub(crate) mod comments;
pub(crate) mod custom_fields;
pub(crate) mod groups;
pub(crate) mod identities;
pub(crate) mod logs;
pub(crate) mod lookups;
pub(crate) mod matrix;
pub(crate) mod notifications;
pub(crate) mod projects;
pub(crate) mod requirement_links;
pub(crate) mod requirements;
pub(crate) mod saved_views;
pub(crate) mod users;
pub(crate) mod verifications;
pub(crate) mod workspaces;

#[cfg(test)]
mod tests {
    use super::PoolStats;
    use std::time::Duration;

    #[test]
    fn utilization_percentage_handles_zero() {
        let stats = PoolStats {
            max_size: 10,
            min_idle: 0,
            current_size: 5,
            available: 5,
        };
        assert_eq!(stats.utilization_percentage(), 50.0);

        let zero_max = PoolStats {
            max_size: 0,
            min_idle: 0,
            current_size: 0,
            available: 0,
        };
        assert_eq!(zero_max.utilization_percentage(), 0.0);
    }

    #[test]
    fn utilization_percentage_full() {
        let stats = PoolStats {
            max_size: 10,
            min_idle: 0,
            current_size: 10,
            available: 0,
        };
        assert_eq!(stats.utilization_percentage(), 100.0);
    }

    #[test]
    fn efficiency_full_available() {
        let stats = PoolStats {
            max_size: 10,
            min_idle: 0,
            current_size: 5,
            available: 5,
        };
        assert_eq!(stats.efficiency(), 100.0);
    }

    #[test]
    fn health_assessment() {
        let healthy = PoolStats {
            max_size: 10,
            min_idle: 0,
            current_size: 5,
            available: 1,
        };
        assert!(healthy.is_healthy());

        let no_available = PoolStats {
            max_size: 10,
            min_idle: 0,
            current_size: 5,
            available: 0,
        };
        assert!(!no_available.is_healthy());

        let too_many = PoolStats {
            max_size: 5,
            min_idle: 0,
            current_size: 6,
            available: 1,
        };
        assert!(!too_many.is_healthy());
    }

    #[test]
    fn active_and_efficiency_metrics() {
        let stats = PoolStats {
            max_size: 12,
            min_idle: 0,
            current_size: 8,
            available: 2,
        };
        assert_eq!(stats.active_connections(), 6);
        assert!((stats.efficiency() - (2.0 / 8.0 * 100.0)).abs() < f64::EPSILON);

        let empty = PoolStats {
            max_size: 12,
            min_idle: 0,
            current_size: 0,
            available: 0,
        };
        assert_eq!(empty.active_connections(), 0);
        assert_eq!(empty.efficiency(), 0.0);
    }

    #[test]
    fn pool_info_creation() {
        use super::PoolInfo;
        use super::PoolStats;

        let stats = PoolStats {
            max_size: 10,
            min_idle: 5,
            current_size: 7,
            available: 2,
        };

        let timeout = Duration::from_secs(30);
        let idle_timeout = Some(Duration::from_secs(600));
        let max_lifetime = Some(Duration::from_secs(1800));

        let info = PoolInfo {
            stats: stats.clone(),
            connection_timeout: timeout,
            idle_timeout,
            max_lifetime,
        };

        assert_eq!(info.stats.max_size, 10);
        assert_eq!(info.stats.min_idle, 5);
        assert_eq!(info.stats.current_size, 7);
        assert_eq!(info.stats.available, 2);
        assert_eq!(info.connection_timeout, timeout);
        assert_eq!(info.idle_timeout, idle_timeout);
        assert_eq!(info.max_lifetime, max_lifetime);
    }

    #[test]
    fn pool_info_with_none_timeouts() {
        use super::PoolInfo;
        use super::PoolStats;

        let stats = PoolStats {
            max_size: 5,
            min_idle: 0,
            current_size: 3,
            available: 1,
        };

        let info = PoolInfo {
            stats,
            connection_timeout: Duration::from_secs(10),
            idle_timeout: None,
            max_lifetime: None,
        };

        assert_eq!(info.idle_timeout, None);
        assert_eq!(info.max_lifetime, None);
    }

    #[test]
    fn pool_stats_clone() {
        let stats = PoolStats {
            max_size: 10,
            min_idle: 5,
            current_size: 7,
            available: 2,
        };

        let cloned = stats.clone();
        assert_eq!(cloned.max_size, stats.max_size);
        assert_eq!(cloned.min_idle, stats.min_idle);
        assert_eq!(cloned.current_size, stats.current_size);
        assert_eq!(cloned.available, stats.available);
    }

    #[test]
    fn pool_stats_debug() {
        let stats = PoolStats {
            max_size: 10,
            min_idle: 5,
            current_size: 7,
            available: 2,
        };

        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("PoolStats"));
    }

    #[test]
    fn pool_info_clone() {
        use super::PoolInfo;
        use super::PoolStats;

        let info = PoolInfo {
            stats: PoolStats {
                max_size: 10,
                min_idle: 5,
                current_size: 7,
                available: 2,
            },
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };

        let cloned = info.clone();
        assert_eq!(cloned.stats.max_size, info.stats.max_size);
        assert_eq!(cloned.connection_timeout, info.connection_timeout);
    }

    #[test]
    fn pool_info_debug() {
        use super::PoolInfo;
        use super::PoolStats;

        let info = PoolInfo {
            stats: PoolStats {
                max_size: 10,
                min_idle: 5,
                current_size: 7,
                available: 2,
            },
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Some(Duration::from_secs(600)),
            max_lifetime: Some(Duration::from_secs(1800)),
        };

        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("PoolInfo"));
    }

    #[test]
    fn utilization_when_current_exceeds_max() {
        let stats = PoolStats {
            max_size: 10,
            min_idle: 0,
            current_size: 12,
            available: 2,
        };
        assert_eq!(stats.utilization_percentage(), 120.0);
    }

    #[test]
    fn active_connections_when_none_available() {
        let stats = PoolStats {
            max_size: 10,
            min_idle: 0,
            current_size: 5,
            available: 0,
        };
        assert_eq!(stats.active_connections(), 5);
    }
}
