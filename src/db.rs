use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::RunQueryDsl;
use std::sync::Arc;
use lazy_static::lazy_static;
use std::time::Duration;

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
    pub fn as_mut(&mut self) -> &mut PgConnection {
        &mut self.inner
    }
    
    /// Get a reference to the inner connection
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

lazy_static! {
    /// Global connection pool instance
    static ref CONNECTION_POOL: Arc<ConnectionPool> = {
        dotenvy::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder()
            .max_size(30) // Increased from 20 for better concurrency
            .min_idle(Some(10))  // Increased from 5 for better performance
            .connection_timeout(Duration::from_secs(30)) // Add timeout
            .idle_timeout(Some(Duration::from_secs(600))) // 10 minutes idle timeout
            .max_lifetime(Some(Duration::from_secs(1800))) // 30 minutes max lifetime
            .build(manager)
            .expect("Failed to create connection pool");
        
        Arc::new(pool)
    };
}

/// Get a connection from the global pool
pub fn get_pooled_connection() -> Result<PooledConn, Box<dyn std::error::Error>> {
    CONNECTION_POOL.get()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// Get a pooled connection wrapper that can be used in place of regular connections
pub fn get_pooled_connection_wrapper() -> Result<PooledConnectionWrapper, Box<dyn std::error::Error>> {
    let pooled_conn = get_pooled_connection()?;
    Ok(PooledConnectionWrapper::new(pooled_conn))
}

/// Get a connection from the pool with proper error handling
/// This function returns a pooled connection wrapper that can be used like a regular connection
pub fn get_connection_pooled_safe() -> Result<PooledConnectionWrapper, Box<dyn std::error::Error>> {
    get_pooled_connection_wrapper()
}

/// Get a connection from the pool (for use outside of Rocket handlers)
/// This function now properly uses the pool instead of falling back to direct connections
pub fn get_connection() -> Result<PooledConnectionWrapper, Box<dyn std::error::Error>> {
    get_pooled_connection_wrapper()
}

/// Get the global connection pool reference
pub fn get_pool() -> &'static ConnectionPool {
    &CONNECTION_POOL
}

/// Get pool statistics
pub fn get_pool_stats() -> PoolStats {
    let pool = get_pool();
    PoolStats {
        max_size: pool.max_size(),
        min_idle: pool.min_idle().unwrap_or(0),
        current_size: pool.state().connections,
        available: pool.state().idle_connections,
    }
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

/// Test the connection pool health
pub fn test_pool_health() -> Result<bool, Box<dyn std::error::Error>> {
    let mut conn = get_pooled_connection()?;
    
    // Test the connection with a simple query
    diesel::sql_query("SELECT 1").execute(&mut conn)?;
    
    Ok(true)
}

/// Get detailed pool information for monitoring
pub fn get_pool_info() -> PoolInfo {
    let stats = get_pool_stats();
    let pool = get_pool();
    
    PoolInfo {
        stats,
        connection_timeout: pool.connection_timeout(),
        idle_timeout: pool.idle_timeout(),
        max_lifetime: pool.max_lifetime(),
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
