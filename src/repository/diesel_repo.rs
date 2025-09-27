use super::errors::RepoError;
use crate::models::*;
use crate::repository::{
    LookupRepository, MatrixRepository, ProjectMembersRepository, ProjectsRepository,
    RequirementsRepository, TestsRepository, UserRepository,
};
use crate::schema;
use diesel::pg::{upsert::excluded, PgConnection};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::RunQueryDsl;
use lazy_static::lazy_static;
use rocket::async_trait;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::time::Duration;

/// Database connection wrapper for use in Rocket handlers
pub type DbConn = rocket_sync_db_pools::diesel::PgConnection;

/// Connection pool type
pub type ConnectionPool = Pool<ConnectionManager<PgConnection>>;
pub type PooledConn = PooledConnection<ConnectionManager<PgConnection>>;
pub type DieselCachedRepo = super::CacheRepository<DieselRepo>;
use rocket::tokio::task;
use diesel::result::Error as DieselError;


#[async_trait]
pub trait DieselRepoLockExt {
    async fn db_read<F, T>(&self, f: F) -> Result<T, RepoError>
    where
        F: FnOnce(&DieselCachedRepo) -> Result<T, RepoError> + Send + 'static,
        T: Send + 'static;

    async fn db_write<F, T>(&self, f: F) -> Result<T, RepoError>
    where
        F: FnOnce(&mut DieselCachedRepo) -> Result<T, RepoError> + Send + 'static,
        T: Send + 'static;
}

#[async_trait]
impl DieselRepoLockExt for Arc<RwLock<DieselCachedRepo>> {
    async fn db_read<F, T>(&self, f: F) -> Result<T, RepoError>
    where
        F: FnOnce(&DieselCachedRepo) -> Result<T, RepoError> + Send + 'static,
        T: Send + 'static,
    {
        let repo = self.clone();
        task::spawn_blocking(move || {
            let guard = repo.read().map_err(|_| RepoError::from(DieselError::NotFound))?;
            f(&*guard)
        })
        .await
        .map_err(|_| RepoError::from(DieselError::NotFound))?
    }

    async fn db_write<F, T>(&self, f: F) -> Result<T, RepoError>
    where
        F: FnOnce(&mut DieselCachedRepo) -> Result<T, RepoError> + Send + 'static,
        T: Send + 'static,
    {
        let repo = self.clone();
        task::spawn_blocking(move || {
            let mut guard = repo.write().map_err(|_| RepoError::from(DieselError::NotFound))?;
            f(&mut *guard)
        })
        .await
        .map_err(|_| RepoError::from(DieselError::NotFound))?
    }
}



lazy_static! {
    /// Shared, mutable, thread-safe repository singleton.
    static ref SHARED_CACHED_REPO: RwLock<DieselCachedRepo> = RwLock::new(
        DieselCachedRepo::new(
            DieselRepo::new(),
            5 * 60, // 5 min
        )
    );
}

impl DieselCachedRepo {
    /// Access the global repo lock (call `.read()` or `.write()` as needed).
    pub fn shared() -> &'static RwLock<DieselCachedRepo> {
        &*SHARED_CACHED_REPO
    }

    /// Convenience helpers if you prefer to grab the guards directly.
    pub fn read() -> RwLockReadGuard<'static, DieselCachedRepo> {
        SHARED_CACHED_REPO.read().expect("repo lock poisoned")
    }

    pub fn write() -> RwLockWriteGuard<'static, DieselCachedRepo> {
        SHARED_CACHED_REPO.write().expect("repo lock poisoned")
    }
}

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

pub struct DieselRepo {
    pool: Arc<ConnectionPool>,
}

impl DieselRepo {
    pub fn new() -> Self {
        Self {
            pool: CONNECTION_POOL.clone(),
        }
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
        let user_id = conn
            .as_mut()
            .transaction::<i32, diesel::result::Error, _>(|conn| {
                let res: User = diesel::insert_into(schema::users::table)
                    .values(new)
                    .get_result(conn)?;
                Ok(res.user_id)
            })?;

        Ok(user_id)
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
                user_password.eq(&user_data.user_password),
                is_admin.eq(user_data.is_admin),
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
                is_admin.eq(user_data.is_admin),
            ))
            .execute(conn.as_mut())?;
        Ok(result > 0)
    }

    fn delete_user(&mut self, id: i32) -> Result<User, RepoError> {
        use crate::schema::users::dsl::*;
        let mut conn = self.get_conn()?;
        let user = users
            .filter(user_id.eq(id))
            .get_result::<User>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(users.filter(user_id.eq(id))).execute(conn.as_mut())?;
        Ok(user)
    }
}

impl ProjectMembersRepository for DieselRepo {
    fn get_members_by_project(&self, pid: i32) -> Result<Vec<ProjectMember>, RepoError> {
        use crate::schema::project_members::dsl::*;

        let mut conn = self.get_conn()?;
        project_members
            .filter(project_id.eq(pid))
            .order(user_id)
            .load::<ProjectMember>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_projects_for_user(&self, uid: i32) -> Result<Vec<ProjectMember>, RepoError> {
        use crate::schema::project_members::dsl::*;

        let mut conn = self.get_conn()?;
        project_members
            .filter(user_id.eq(uid))
            .order(project_id)
            .load::<ProjectMember>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn add_project_member(&mut self, new: &NewProjectMember) -> Result<(), RepoError> {
        use crate::schema::project_members::dsl::*;

        let mut conn = self.get_conn()?;
        diesel::insert_into(project_members)
            .values(new)
            .on_conflict((project_id, user_id))
            .do_update()
            .set((
                role.eq(excluded(role)),
                updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(conn.as_mut())?;
        Ok(())
    }

    fn update_project_member_role(
        &mut self,
        pid: i32,
        uid: i32,
        new_role: i32,
    ) -> Result<(), RepoError> {
        use crate::schema::project_members::dsl::*;

        let mut conn = self.get_conn()?;
        let affected = diesel::update(
            project_members
                .filter(project_id.eq(pid))
                .filter(user_id.eq(uid)),
        )
        .set((
            role.eq(new_role),
            updated_at.eq(chrono::Utc::now().naive_utc()),
        ))
        .execute(conn.as_mut())?;

        if affected == 0 {
            Err(RepoError::NotFound)
        } else {
            Ok(())
        }
    }

    fn remove_project_member(&mut self, pid: i32, uid: i32) -> Result<(), RepoError> {
        use crate::schema::project_members::dsl::*;

        let mut conn = self.get_conn()?;
        let affected = diesel::delete(
            project_members
                .filter(project_id.eq(pid))
                .filter(user_id.eq(uid)),
        )
        .execute(conn.as_mut())?;

        if affected == 0 {
            Err(RepoError::NotFound)
        } else {
            Ok(())
        }
    }
}

impl LookupRepository for DieselRepo {
    fn get_status_all(&self) -> Result<Vec<Status>, RepoError> {
        // For backward compatibility, return requirement status as status
        let req_statuses = self.get_requirement_status_all()?;
        Ok(req_statuses
            .into_iter()
            .map(|rs| Status {
                st_id: rs.req_st_id,
                st_title: rs.req_st_title,
                st_description: rs.req_st_description,
                st_short_name: rs.req_st_short_name,
            })
            .collect())
    }

    fn get_status_by_id(&self, id: i32) -> Result<Status, RepoError> {
        // For backward compatibility, get from requirement status
        let req_status = self.get_requirement_status_by_id(id)?;
        Ok(Status {
            st_id: req_status.req_st_id,
            st_title: req_status.req_st_title,
            st_description: req_status.req_st_description,
            st_short_name: req_status.req_st_short_name,
        })
    }

    fn get_requirement_status_all(&self) -> Result<Vec<RequirementStatus>, RepoError> {
        use schema::requirement_status::dsl::*;
        let mut conn = self.get_conn()?;
        requirement_status
            .order(req_st_id)
            .load::<RequirementStatus>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_requirement_status_by_id(&self, id: i32) -> Result<RequirementStatus, RepoError> {
        use schema::requirement_status::dsl::*;
        let mut conn = self.get_conn()?;
        requirement_status
            .filter(req_st_id.eq(id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_test_status_all(&self) -> Result<Vec<TestStatus>, RepoError> {
        use schema::test_status::dsl::*;
        let mut conn = self.get_conn()?;
        test_status
            .order(test_st_id)
            .load::<TestStatus>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_test_status_by_id(&self, id: i32) -> Result<TestStatus, RepoError> {
        use schema::test_status::dsl::*;
        let mut conn = self.get_conn()?;
        test_status
            .filter(test_st_id.eq(id))
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

    fn delete_category(&mut self, id: i32) -> Result<Category, RepoError> {
        use schema::categories::dsl::*;
        let mut conn = self.get_conn()?;
        let cat = categories
            .filter(cat_id.eq(id))
            .get_result::<Category>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(categories.filter(cat_id.eq(id))).execute(conn.as_mut())?;
        Ok(cat)
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

    fn delete_applicability(&mut self, id: i32) -> Result<Applicability, RepoError> {
        use schema::applicability::dsl::*;
        let mut conn = self.get_conn()?;
        let app = applicability
            .filter(app_id.eq(id))
            .get_result::<Applicability>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(applicability.filter(app_id.eq(id))).execute(conn.as_mut())?;
        Ok(app)
    }

    fn create_status(&mut self, new: &NewStatus) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        let res: RequirementStatus = diesel::insert_into(schema::requirement_status::table)
            .values(new)
            .get_result(conn.as_mut())?;
        Ok(res.req_st_id)
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

    fn delete_requirement(&mut self, id: i32) -> Result<Requirement, RepoError> {
        use crate::schema::requirements::dsl::*;
        let mut conn = self.get_conn()?;
        let req = requirements
            .filter(req_id.eq(id))
            .get_result::<Requirement>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(requirements.filter(req_id.eq(id))).execute(conn.as_mut())?;
        Ok(req)
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

    fn get_tests_for_requirement(&self, rid: i32) -> Result<Vec<Test>, RepoError> {
        use schema::matrix::dsl::{matrix, matrix_req_id, matrix_test_id};
        use schema::tests::dsl as t;
        let mut conn = self.get_conn()?;
        matrix
            .filter(matrix_req_id.eq(rid))
            .inner_join(t::tests.on(matrix_test_id.eq(t::test_id)))
            .select((
                t::test_id,
                t::test_name,
                t::test_description,
                t::test_source,
                t::test_status,
                t::test_reference,
                t::test_parent,
                t::project_id,
            ))
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

    fn delete_test(&mut self, id: i32) -> Result<Test, RepoError> {
        use crate::schema::tests::dsl::*;
        let mut conn = self.get_conn()?;
        let test = tests
            .filter(test_id.eq(id))
            .get_result::<Test>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(tests.filter(test_id.eq(id))).execute(conn.as_mut())?;
        Ok(test)
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

    fn delete_project(&mut self, project_id_param: i32) -> Result<Project, RepoError> {
        use schema::projects::dsl::*;
        let mut conn = self.get_conn()?;
        let proj = projects
            .filter(project_id.eq(project_id_param))
            .get_result::<Project>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(projects.filter(project_id.eq(project_id_param))).execute(conn.as_mut())?;
        Ok(proj)
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

#[cfg(test)]
mod tests {
    use super::PoolStats;

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
}
