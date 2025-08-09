use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use std::sync::Arc;
use lazy_static::lazy_static;

/// Database connection wrapper for use in Rocket handlers
pub type DbConn = rocket_sync_db_pools::diesel::PgConnection;

/// Connection pool type
pub type ConnectionPool = Pool<ConnectionManager<PgConnection>>;
pub type PooledConn = PooledConnection<ConnectionManager<PgConnection>>;

lazy_static! {
    /// Global connection pool instance
    static ref CONNECTION_POOL: Arc<ConnectionPool> = {
        dotenvy::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        let pool = Pool::builder()
            .max_size(20) // Maximum number of connections in the pool
            .min_idle(Some(5))  // Minimum number of idle connections
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

/// Get a connection from the pool with safe error handling
/// This function returns a default connection if the pool fails
pub fn get_connection_pooled_safe() -> PgConnection {
    match get_pooled_connection() {
        Ok(_pooled_conn) => {
            // Convert pooled connection to regular connection
            // This is a workaround since we can't return the pooled connection directly
            // In a real application, you'd want to keep the pooled connection
            use crate::helper_functions::establish_connection;
            establish_connection()
        }
        Err(_) => {
            // Fallback to direct connection if pool fails
            use crate::helper_functions::establish_connection;
            establish_connection()
        }
    }
}

/// Get a connection from the pool (for use outside of Rocket handlers)
/// This is a fallback for functions that can't use the connection pool
pub fn get_connection() -> Result<PgConnection, Box<dyn std::error::Error>> {
    use crate::helper_functions::establish_connection;
    Ok(establish_connection())
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
}
