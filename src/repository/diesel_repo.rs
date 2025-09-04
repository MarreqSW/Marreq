use super::errors::RepoError;
use crate::models::*;
use crate::repository::{
    LookupRepository, MatrixRepository, ProjectsRepository, RequirementsRepository,
    TestsRepository, UserRepository,
};
use crate::schema;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::RunQueryDsl;
use lazy_static::lazy_static;
use std::sync::Arc;
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
    CONNECTION_POOL
        .get()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

/// Get a pooled connection wrapper that can be used in place of regular connections
pub fn get_pooled_connection_wrapper() -> Result<PooledConnectionWrapper, Box<dyn std::error::Error>> {
    let pooled_conn = get_pooled_connection()?;
    Ok(PooledConnectionWrapper::new(pooled_conn))
}

/// Get a connection from the pool with proper error handling
/// This function returns a pooled connection wrapper that can be used like a regular connection
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

pub struct DieselRepo {
    pool: &'static ConnectionPool,
}

impl DieselRepo {
    pub fn new() -> Self {
        Self {
            pool: get_pool(),
        }
    }

    fn get_conn(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.pool
            .get()
            .map(PooledConnectionWrapper::new)
            .map_err(|e| RepoError::Pool(e.to_string()))
    }

    pub fn pool_stats(&self) -> PoolStats {
        get_pool_stats()
    }

    pub fn pool_info(&self) -> PoolInfo {
        get_pool_info()
    }

    pub fn test_pool_health(&self) -> Result<bool, RepoError> {
        test_pool_health().map_err(|e| RepoError::Pool(e.to_string()))
    }
}

impl UserRepository for DieselRepo {
    fn get_users_all(&self) -> Result<Vec<User>, RepoError> {
        use schema::users::dsl::*;
        let mut conn = self.get_conn()?;
        users
            .order(user_id)
            .load::<User>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_user_by_id(&self, idv: i32) -> Result<User, RepoError> {
        use schema::users::dsl::*;
        let mut conn = self.get_conn()?;

        users
            .filter(user_id.eq(idv))
            .first::<User>(conn.as_mut()) // <-- use inner PgConnection
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_user_by_username(&self, uname: &str) -> Result<Option<User>, RepoError> {
        use crate::schema::users::dsl::*;
        let mut conn = self.get_conn()?;

        users
            .filter(user_username.eq(uname))
            .first::<User>(conn.as_mut())
            .optional()
            .map_err(|e| e.into())
    }

    fn update_user_password(&mut self, id: i32, new_hash: &str) -> Result<(), RepoError> {
        use crate::schema::users::dsl::*;
        let mut conn = self.get_conn()?;

        let affected = diesel::update(users.filter(user_id.eq(id)))
            .set(user_password.eq(new_hash))
            .execute(conn.as_mut())?;

        if affected == 1 {
            Ok(())
        } else if affected == 0 {
            Err(RepoError::NotFound)
        } else {
            Err(RepoError::Db(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::Unknown,
                Box::new(format!("updated {} rows for user_id={}", affected, id)),
            )))
        }
    }

    fn insert_user(&mut self, new: &NewUser) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        let res: User = diesel::insert_into(schema::users::table)
            .values(new)
            .get_result(conn.as_mut())?;
        Ok(res.user_id)
    }

    fn update_user(&mut self, user_data: &NewUser) -> Result<bool, RepoError> {
        use crate::schema::users::dsl::*;
        let mut conn = self.get_conn()?;
        let user_id_value = user_data
            .user_id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let result = diesel::update(users.filter(user_id.eq(user_id_value)))
            .set((
                user_name.eq(&user_data.user_name),
                user_username.eq(&user_data.user_username),
                user_email.eq(&user_data.user_email),
                user_level.eq(user_data.user_level),
                user_password.eq(&user_data.user_password),
            ))
            .execute(conn.as_mut())?;
        Ok(result > 0)
    }

    fn update_user_without_password(&mut self, user_data: &UpdateUser) -> Result<bool, RepoError> {
        use crate::schema::users::dsl::*;
        let mut conn = self.get_conn()?;
        let user_id_value = user_data
            .user_id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let result = diesel::update(users.filter(user_id.eq(user_id_value)))
            .set((
                user_name.eq(&user_data.user_name),
                user_username.eq(&user_data.user_username),
                user_email.eq(&user_data.user_email),
                user_level.eq(user_data.user_level),
            ))
            .execute(conn.as_mut())?;
        Ok(result > 0)
    }

    fn delete_user(&mut self, id: i32) -> Result<bool, RepoError> {
        use crate::schema::users::dsl::*;
        let mut conn = self.get_conn()?;
        let deleted = diesel::delete(users.filter(user_id.eq(id))).execute(conn.as_mut())?;
        Ok(deleted > 0)
    }
}

impl LookupRepository for DieselRepo {
    fn get_status_all(&self) -> Result<Vec<Status>, RepoError> {
        use schema::status::dsl::*;
        let mut conn = self.get_conn()?;
        status
            .order(st_id)
            .load::<Status>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_status_by_id(&self, id: i32) -> Result<Status, RepoError> {
        use schema::status::dsl::*;
        let mut conn = self.get_conn()?;
        status
            .filter(st_id.eq(id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_categories_all(&self) -> Result<Vec<Category>, RepoError> {
        use schema::categories::dsl::*;
        let mut conn = self.get_conn()?;
        categories
            .order(cat_id)
            .load::<Category>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_category_by_id(&self, id: i32) -> Result<Category, RepoError> {
        use schema::categories::dsl::*;
        let mut conn = self.get_conn()?;
        categories
            .filter(cat_id.eq(id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_categories_by_project(&self, pid: i32) -> Result<Vec<Category>, RepoError> {
        use schema::categories::dsl::*;
        let mut conn = self.get_conn()?;
        categories
            .filter(project_id.eq(pid))
            .load::<Category>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_applicability_all(&self) -> Result<Vec<Applicability>, RepoError> {
        use schema::applicability::dsl::*;
        let mut conn = self.get_conn()?;
        applicability
            .order(app_id)
            .load::<Applicability>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_applicability_by_id(&self, id: i32) -> Result<Applicability, RepoError> {
        use schema::applicability::dsl::*;
        let mut conn = self.get_conn()?;
        applicability
            .filter(app_id.eq(id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_applicability_by_project(&self, pid: i32) -> Result<Vec<Applicability>, RepoError> {
        use schema::applicability::dsl::*;
        let mut conn = self.get_conn()?;
        applicability
            .filter(project_id.eq(pid))
            .load::<Applicability>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_verification_all(&self) -> Result<Vec<Verification>, RepoError> {
        use schema::verification::dsl::*;
        let mut conn = self.get_conn()?;
        verification
            .order(verification_id)
            .load::<Verification>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_verification_by_id(&self, id: i32) -> Result<Verification, RepoError> {
        use schema::verification::dsl::*;
        let mut conn = self.get_conn()?;
        verification
            .filter(verification_id.eq(id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_verification_by_project(&self, pid: i32) -> Result<Vec<Verification>, RepoError> {
        use schema::verification::dsl::*;
        let mut conn = self.get_conn()?;
        verification
            .filter(project_id.eq(pid))
            .order(verification_id)
            .load::<Verification>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn insert_new_category(&mut self, new: &NewCategory) -> Result<i32, RepoError> {
        use schema::categories::dsl::*;
        let mut conn = self.get_conn()?;
        let result = diesel::insert_into(categories)
            .values(new)
            .get_result::<Category>(conn.as_mut())?;
        Ok(result.cat_id)
    }

    fn edit_category(&mut self, new: &NewCategory) -> Result<bool, RepoError> {
        use schema::categories::dsl::*;
        let mut conn = self.get_conn()?;
        let category_id = new
            .cat_id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let updated = diesel::update(categories.filter(cat_id.eq(category_id)))
            .set((
                cat_title.eq(&new.cat_title),
                cat_description.eq(&new.cat_description),
                cat_tag.eq(&new.cat_tag),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_category(&mut self, id: i32) -> Result<bool, RepoError> {
        use schema::categories::dsl::*;
        let mut conn = self.get_conn()?;
        let deleted = diesel::delete(categories.filter(cat_id.eq(id))).execute(conn.as_mut())?;
        Ok(deleted > 0)
    }

    fn insert_new_applicability(&mut self, new: &NewApplicability) -> Result<i32, RepoError> {
        use schema::applicability::dsl::*;
        let mut conn = self.get_conn()?;
        let result = diesel::insert_into(applicability)
            .values(new)
            .get_result::<Applicability>(conn.as_mut())?;
        Ok(result.app_id)
    }

    fn edit_applicability(&mut self, new: &NewApplicability) -> Result<bool, RepoError> {
        use schema::applicability::dsl::*;
        let mut conn = self.get_conn()?;
        let app_id_val = new
            .app_id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let updated = diesel::update(applicability.filter(app_id.eq(app_id_val)))
            .set((
                app_title.eq(&new.app_title),
                app_description.eq(&new.app_description),
                app_tag.eq(&new.app_tag),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_applicability(&mut self, id: i32) -> Result<bool, RepoError> {
        use schema::applicability::dsl::*;
        let mut conn = self.get_conn()?;
        let deleted = diesel::delete(applicability.filter(app_id.eq(id))).execute(conn.as_mut())?;
        Ok(deleted > 0)
    }

    fn create_status(&mut self, new: &NewStatus) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        let res: Status = diesel::insert_into(schema::status::table)
            .values(new)
            .get_result(conn.as_mut())?;
        Ok(res.st_id)
    }
}

impl RequirementsRepository for DieselRepo {
    fn get_requirement_by_id(&self, id: i32) -> Result<Requirement, RepoError> {
        use schema::requirements::dsl::*;
        let mut conn = self.get_conn()?;
        requirements
            .filter(req_id.eq(id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_requirements_all(&self) -> Result<Vec<Requirement>, RepoError> {
        use schema::requirements::dsl::*;
        let mut conn = self.get_conn()?;
        requirements
            .order(req_id)
            .load::<Requirement>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_requirements_by_project(&self, project: i32) -> Result<Vec<Requirement>, RepoError> {
        use schema::requirements::dsl::*;
        let mut conn = self.get_conn()?;
        requirements
            .filter(schema::requirements::project_id.eq(project))
            .load::<Requirement>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn insert_new_requirement(&mut self, new: &NewRequirement) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        let res: Requirement = diesel::insert_into(schema::requirements::table)
            .values(new)
            .get_result(conn.as_mut())?;
        Ok(res.req_id)
    }

    fn edit_requirement(&mut self, new: &NewRequirement) -> Result<bool, RepoError> {
        use crate::schema::requirements::dsl::*;
        let mut conn = self.get_conn()?;
        let id_val = new
            .req_id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        diesel::update(requirements.filter(req_id.eq(id_val)))
            .set(new)
            .execute(conn.as_mut())
            .map(|_| true)
            .map_err(|e| e.into())
    }

    fn delete_requirement(&mut self, id: i32) -> Result<bool, RepoError> {
        use crate::schema::requirements::dsl::*;
        let mut conn = self.get_conn()?;
        let deleted = diesel::delete(requirements.filter(req_id.eq(id))).execute(conn.as_mut())?;
        Ok(deleted > 0)
    }

    fn update_requirement(&mut self, req: i32) -> Result<(), RepoError> {
        use crate::schema::requirements::dsl::*;
        use diesel::dsl::now;
        let mut conn = self.get_conn()?;
        diesel::update(requirements)
            .filter(req_id.eq(req))
            .set(req_update_date.eq(now))
            .execute(conn.as_mut())?;
        Ok(())
    }
}

impl TestsRepository for DieselRepo {
    fn get_test_by_id(&self, id: i32) -> Result<Test, RepoError> {
        use schema::tests::dsl::*;
        let mut conn = self.get_conn()?;
        tests
            .filter(test_id.eq(id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_tests_all(&self) -> Result<Vec<Test>, RepoError> {
        use schema::tests::dsl::*;
        let mut conn = self.get_conn()?;
        tests
            .order(test_id)
            .load::<Test>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_tests_by_project(&self, project: i32) -> Result<Vec<Test>, RepoError> {
        use schema::tests::dsl::*;
        let mut conn = self.get_conn()?;
        tests
            .filter(schema::tests::project_id.eq(project))
            .load::<Test>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_requirements_for_test(&self, tid: i32) -> Result<Vec<Requirement>, RepoError> {
        use schema::matrix::dsl::*;
        use schema::requirements::dsl::*;
        let mut conn = self.get_conn()?;
        matrix
            .filter(matrix_test_id.eq(tid))
            .inner_join(requirements.on(matrix_req_id.eq(req_id)))
            .select((
                req_id,
                req_title,
                req_description,
                req_verification,
                req_current_status,
                req_author,
                req_reviewer,
                req_link,
                req_reference,
                req_category,
                req_parent,
                req_creation_date,
                req_update_date,
                req_deadline_date,
                req_applicability,
                req_justification,
                schema::requirements::project_id,
            ))
            .load::<Requirement>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn insert_test(&mut self, new: &NewTest) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        let res: Test = diesel::insert_into(schema::tests::table)
            .values(new)
            .get_result(conn.as_mut())?;
        Ok(res.test_id)
    }

    fn edit_test(&mut self, new: &NewTest) -> Result<bool, RepoError> {
        use crate::schema::tests::dsl::*;
        let mut conn = self.get_conn()?;
        let test_id_value = new
            .test_id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let updated = diesel::update(tests.filter(test_id.eq(test_id_value)))
            .set((
                test_name.eq(&new.test_name),
                test_description.eq(&new.test_description),
                test_source.eq(&new.test_source),
                test_status.eq(&new.test_status),
                test_parent.eq(&new.test_parent),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_test(&mut self, id: i32) -> Result<bool, RepoError> {
        use crate::schema::tests::dsl::*;
        let mut conn = self.get_conn()?;
        let deleted = diesel::delete(tests.filter(test_id.eq(id))).execute(conn.as_mut())?;
        Ok(deleted > 0)
    }

    fn update_test_requirement_links(
        &mut self,
        test_id_val: i32,
        requirement_ids: &[i32],
    ) -> Result<(), RepoError> {
        use schema::matrix::dsl::*;
        let mut conn = self.get_conn()?;
        diesel::delete(matrix.filter(matrix_test_id.eq(test_id_val))).execute(conn.as_mut())?;
        for req_id in requirement_ids {
            let matrix_item = NewMatrix {
                matrix_req_id: *req_id,
                matrix_test_id: test_id_val,
                project_id: 1,
            };
            diesel::insert_into(schema::matrix::table)
                .values(&matrix_item)
                .execute(conn.as_mut())?;
        }
        Ok(())
    }
}

impl ProjectsRepository for DieselRepo {
    fn get_projects_all(&self) -> Result<Vec<Project>, RepoError> {
        use schema::projects::dsl::*;
        let mut conn = self.get_conn()?;
        projects
            .load::<Project>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_project_by_id(&self, id: i32) -> Result<Project, RepoError> {
        use schema::projects::dsl::*;
        let mut conn = self.get_conn()?;
        projects
            .filter(project_id.eq(id))
            .first::<Project>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn insert_new_project(&mut self, new: &NewProject) -> Result<i32, RepoError> {
        use schema::projects::dsl::*;
        let mut conn = self.get_conn()?;
        let result = diesel::insert_into(projects)
            .values(new)
            .get_result::<Project>(conn.as_mut())?;
        Ok(result.project_id)
    }

    fn edit_project(
        &mut self,
        project_id_param: i32,
        update: &UpdateProject,
    ) -> Result<bool, RepoError> {
        use schema::projects::dsl::*;
        let mut conn = self.get_conn()?;
        let updated = diesel::update(projects.filter(project_id.eq(project_id_param)))
            .set((
                project_name.eq(&update.project_name),
                project_description.eq(&update.project_description),
                project_status.eq(&update.project_status),
                project_owner_id.eq(&update.project_owner_id),
                project_update_date.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_project(&mut self, project_id_param: i32) -> Result<bool, RepoError> {
        use schema::projects::dsl::*;
        let mut conn = self.get_conn()?;
        let deleted = diesel::delete(projects.filter(project_id.eq(project_id_param)))
            .execute(conn.as_mut())?;
        Ok(deleted > 0)
    }
}

impl MatrixRepository for DieselRepo {
    fn get_matrix_by_project(&self, pid: i32) -> Result<Vec<Matrix>, RepoError> {
        use schema::matrix::dsl::*;
        let mut conn = self.get_conn()?;
        matrix
            .filter(project_id.eq(pid))
            .load::<Matrix>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn insert_new_matrix_item(&mut self, new: &NewMatrix) -> Result<(), RepoError> {
        let mut conn = self.get_conn()?;
        diesel::insert_into(schema::matrix::table)
            .values(new)
            .execute(conn.as_mut())?;
        Ok(())
    }
}
