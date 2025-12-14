use super::errors::RepoError;
use crate::models::entities::{
    Applicability, Category, Log, MatrixLink, Project, ProjectMember, Requirement,
    RequirementStatus, TestCase, TestStatus, User, VerificationMethod,
};
use crate::models::forms::{
    NewApplicability, NewCategory, NewLog, NewMatrixLink, NewProject, NewProjectMember,
    NewRequirement, NewRequirementStatus, NewTestCase, NewTestStatus, NewUser,
    NewVerificationMethod, UpdateProject, UpdateUser,
};
use crate::repository::{
    LookupRepository, MatrixRepository, ProjectMembersRepository, ProjectsRepository,
    RequirementsRepository, TestsCaseRepository, UserRepository,
};
use crate::schema;
use diesel::pg::{upsert::excluded, PgConnection};
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::{Connection, ExpressionMethods, JoinOnDsl, OptionalExtension, QueryDsl, RunQueryDsl};
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
        use schema::users::dsl;
        let mut conn = self.get_conn()?;
        dsl::users
            .order(dsl::id)
            .load::<User>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_user_by_id(&self, user_id: i32) -> Result<User, RepoError> {
        use schema::users::dsl;
        let mut conn = self.get_conn()?;

        dsl::users
            .filter(dsl::id.eq(user_id))
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
        use crate::schema::users::dsl;
        let mut conn = self.get_conn()?;

        dsl::users
            .filter(dsl::username.eq(uname))
            .first::<User>(conn.as_mut())
            .optional()
            .map_err(|e| e.into())
    }

    fn update_user_password(&mut self, user_id: i32, new_hash: &str) -> Result<(), RepoError> {
        use crate::schema::users::dsl;
        let mut conn = self.get_conn()?;

        let affected = diesel::update(dsl::users.filter(dsl::id.eq(user_id)))
            .set(dsl::password_hash.eq(new_hash))
            .execute(conn.as_mut())?;

        if affected == 1 {
            Ok(())
        } else if affected == 0 {
            Err(RepoError::NotFound)
        } else {
            Err(RepoError::Db(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::Unknown,
                Box::new(format!("updated {} rows for id={}", affected, user_id)),
            )))
        }
    }

    fn insert_user(&mut self, new: &NewUser) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        let id = conn
            .as_mut()
            .transaction::<i32, diesel::result::Error, _>(|conn| {
                let res: User = diesel::insert_into(schema::users::table)
                    .values(new)
                    .get_result(conn)?;
                Ok(res.id)
            })?;

        Ok(id)
    }

    fn update_user(&mut self, user_data: &NewUser) -> Result<bool, RepoError> {
        use crate::schema::users::dsl;
        let mut conn = self.get_conn()?;
        let user_id_value = user_data
            .id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let result = diesel::update(dsl::users.filter(dsl::id.eq(user_id_value)))
            .set((
                dsl::name.eq(&user_data.name),
                dsl::username.eq(&user_data.username),
                dsl::email.eq(&user_data.email),
                dsl::password_hash.eq(&user_data.password_hash),
                dsl::is_admin.eq(user_data.is_admin),
            ))
            .execute(conn.as_mut())?;
        Ok(result > 0)
    }

    fn update_user_without_password(&mut self, user_data: &UpdateUser) -> Result<bool, RepoError> {
        use crate::schema::users::dsl;
        let mut conn = self.get_conn()?;
        let user_id_value = user_data
            .id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let result = diesel::update(dsl::users.filter(dsl::id.eq(user_id_value)))
            .set((
                dsl::name.eq(&user_data.name),
                dsl::username.eq(&user_data.username),
                dsl::email.eq(&user_data.email),
                dsl::is_admin.eq(user_data.is_admin),
            ))
            .execute(conn.as_mut())?;
        Ok(result > 0)
    }

    fn delete_user(&mut self, user_id: i32) -> Result<User, RepoError> {
        use crate::schema::users::dsl;
        let mut conn = self.get_conn()?;
        let user = dsl::users
            .filter(dsl::id.eq(user_id))
            .get_result::<User>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(dsl::users.filter(dsl::id.eq(user_id))).execute(conn.as_mut())?;
        Ok(user)
    }
}

impl ProjectMembersRepository for DieselRepo {
    fn get_members_by_project(&self, project_id: i32) -> Result<Vec<ProjectMember>, RepoError> {
        use crate::schema::project_members::dsl;

        let mut conn = self.get_conn()?;
        dsl::project_members
            .filter(dsl::project_id.eq(project_id))
            .order(dsl::user_id)
            .load::<ProjectMember>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_projects_for_user(&self, user_id: i32) -> Result<Vec<ProjectMember>, RepoError> {
        use crate::schema::project_members::dsl;

        let mut conn = self.get_conn()?;
        dsl::project_members
            .filter(dsl::user_id.eq(user_id))
            .order(dsl::project_id)
            .load::<ProjectMember>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn add_project_member(&mut self, new: &NewProjectMember) -> Result<(), RepoError> {
        use crate::schema::project_members::dsl;

        let mut conn = self.get_conn()?;
        diesel::insert_into(dsl::project_members)
            .values(new)
            .on_conflict((dsl::project_id, dsl::user_id))
            .do_update()
            .set((
                dsl::role.eq(excluded(dsl::role)),
                dsl::updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(conn.as_mut())?;
        Ok(())
    }

    fn update_project_member_role(
        &mut self,
        project_id: i32,
        user_id: i32,
        new_role: i32,
    ) -> Result<(), RepoError> {
        use crate::schema::project_members::dsl;

        let mut conn = self.get_conn()?;
        let affected = diesel::update(
            dsl::project_members
                .filter(dsl::project_id.eq(project_id))
                .filter(dsl::user_id.eq(user_id)),
        )
        .set((
            dsl::role.eq(new_role),
            dsl::updated_at.eq(chrono::Utc::now().naive_utc()),
        ))
        .execute(conn.as_mut())?;

        if affected == 0 {
            Err(RepoError::NotFound)
        } else {
            Ok(())
        }
    }

    fn remove_project_member(&mut self, project_id: i32, user_id: i32) -> Result<(), RepoError> {
        use crate::schema::project_members::dsl;

        let mut conn = self.get_conn()?;
        let affected = diesel::delete(
            dsl::project_members
                .filter(dsl::project_id.eq(project_id))
                .filter(dsl::user_id.eq(user_id)),
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
    fn get_requirement_status_all(&self) -> Result<Vec<RequirementStatus>, RepoError> {
        use schema::requirement_status::dsl;
        let mut conn = self.get_conn()?;
        dsl::requirement_status
            .order(dsl::id)
            .load::<RequirementStatus>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_requirement_status_by_id(&self, status_id: i32) -> Result<RequirementStatus, RepoError> {
        use schema::requirement_status::dsl;
        let mut conn = self.get_conn()?;
        dsl::requirement_status
            .filter(dsl::id.eq(status_id))
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
        use schema::test_status::dsl;
        let mut conn = self.get_conn()?;
        dsl::test_status
            .order(dsl::id)
            .load::<TestStatus>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_test_status_by_id(&self, status_id: i32) -> Result<TestStatus, RepoError> {
        use schema::test_status::dsl;
        let mut conn = self.get_conn()?;
        dsl::test_status
            .filter(dsl::id.eq(status_id))
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
        use schema::categories::dsl;
        let mut conn = self.get_conn()?;
        dsl::categories
            .order(dsl::id)
            .load::<Category>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_category_by_id(&self, category_id: i32) -> Result<Category, RepoError> {
        use schema::categories::dsl;
        let mut conn = self.get_conn()?;
        dsl::categories
            .filter(dsl::id.eq(category_id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_categories_by_project(&self, project_id: i32) -> Result<Vec<Category>, RepoError> {
        use schema::categories::dsl;
        let mut conn = self.get_conn()?;
        dsl::categories
            .filter(dsl::project_id.eq(project_id))
            .load::<Category>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_applicability_all(&self) -> Result<Vec<Applicability>, RepoError> {
        use schema::applicability::dsl;
        let mut conn = self.get_conn()?;
        dsl::applicability
            .order(dsl::id)
            .load::<Applicability>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_applicability_by_id(&self, applicability_id: i32) -> Result<Applicability, RepoError> {
        use schema::applicability::dsl;
        let mut conn = self.get_conn()?;
        dsl::applicability
            .filter(dsl::id.eq(applicability_id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_applicability_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<Applicability>, RepoError> {
        use schema::applicability::dsl;
        let mut conn = self.get_conn()?;
        dsl::applicability
            .filter(dsl::project_id.eq(project_id))
            .load::<Applicability>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_verification_all(&self) -> Result<Vec<VerificationMethod>, RepoError> {
        use schema::verification::dsl;
        let mut conn = self.get_conn()?;
        dsl::verification
            .order(dsl::id)
            .load::<VerificationMethod>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_verification_by_id(
        &self,
        verification_id: i32,
    ) -> Result<VerificationMethod, RepoError> {
        use schema::verification::dsl;
        let mut conn = self.get_conn()?;
        dsl::verification
            .filter(dsl::id.eq(verification_id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_verification_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<VerificationMethod>, RepoError> {
        use schema::verification::dsl;
        let mut conn = self.get_conn()?;
        dsl::verification
            .filter(dsl::project_id.eq(project_id))
            .order(dsl::id)
            .load::<VerificationMethod>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn insert_new_verification(&mut self, new: &NewVerificationMethod) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        let result = diesel::insert_into(schema::verification::table)
            .values(new)
            .get_result::<VerificationMethod>(conn.as_mut())?;
        Ok(result.id)
    }

    fn insert_new_category(&mut self, new: &NewCategory) -> Result<i32, RepoError> {
        use schema::categories::dsl;
        let mut conn = self.get_conn()?;
        let result = diesel::insert_into(dsl::categories)
            .values(new)
            .get_result::<Category>(conn.as_mut())?;
        Ok(result.id)
    }

    fn edit_category(&mut self, new: &NewCategory) -> Result<bool, RepoError> {
        use schema::categories::dsl;
        let mut conn = self.get_conn()?;
        let category_id = new
            .id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let updated = diesel::update(dsl::categories.filter(dsl::id.eq(category_id)))
            .set((
                dsl::title.eq(&new.title),
                dsl::description.eq(&new.description),
                dsl::tag.eq(&new.tag),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_category(&mut self, category_id: i32) -> Result<Category, RepoError> {
        use schema::categories::dsl;
        let mut conn = self.get_conn()?;
        let cat = dsl::categories
            .filter(dsl::id.eq(category_id))
            .get_result::<Category>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(dsl::categories.filter(dsl::id.eq(category_id))).execute(conn.as_mut())?;
        Ok(cat)
    }

    fn insert_new_applicability(&mut self, new: &NewApplicability) -> Result<i32, RepoError> {
        use schema::applicability::dsl;
        let mut conn = self.get_conn()?;
        let result = diesel::insert_into(dsl::applicability)
            .values(new)
            .get_result::<Applicability>(conn.as_mut())?;
        Ok(result.id)
    }

    fn edit_applicability(&mut self, new: &NewApplicability) -> Result<bool, RepoError> {
        use schema::applicability::dsl;
        let mut conn = self.get_conn()?;
        let app_id_val = new
            .id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let updated = diesel::update(dsl::applicability.filter(dsl::id.eq(app_id_val)))
            .set((
                dsl::title.eq(&new.title),
                dsl::description.eq(&new.description),
                dsl::tag.eq(&new.tag),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_applicability(&mut self, applicability_id: i32) -> Result<Applicability, RepoError> {
        use schema::applicability::dsl;
        let mut conn = self.get_conn()?;
        let app = dsl::applicability
            .filter(dsl::id.eq(applicability_id))
            .get_result::<Applicability>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(dsl::applicability.filter(dsl::id.eq(applicability_id)))
            .execute(conn.as_mut())?;
        Ok(app)
    }

    fn create_requirement_status(&mut self, new: &NewRequirementStatus) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        let res: RequirementStatus = diesel::insert_into(schema::requirement_status::table)
            .values(new)
            .get_result(conn.as_mut())?;
        Ok(res.id)
    }

    fn create_test_status(&mut self, new: &NewTestStatus) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        let res: TestStatus = diesel::insert_into(schema::test_status::table)
            .values(new)
            .get_result(conn.as_mut())?;
        Ok(res.id)
    }
}

impl RequirementsRepository for DieselRepo {
    fn get_requirement_by_id(&self, requirement_id: i32) -> Result<Requirement, RepoError> {
        use schema::requirements::dsl;
        let mut conn = self.get_conn()?;
        dsl::requirements
            .filter(dsl::id.eq(requirement_id))
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
        use schema::requirements::dsl;
        let mut conn = self.get_conn()?;
        dsl::requirements
            .order(dsl::id)
            .load::<Requirement>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_requirements_by_project(&self, project_id: i32) -> Result<Vec<Requirement>, RepoError> {
        use schema::requirements::dsl;
        let mut conn = self.get_conn()?;
        dsl::requirements
            .filter(dsl::project_id.eq(project_id))
            .load::<Requirement>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn insert_new_requirement(&mut self, new: &NewRequirement) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        let res: Requirement = diesel::insert_into(schema::requirements::table)
            .values(new)
            .get_result(conn.as_mut())?;
        Ok(res.id)
    }

    fn edit_requirement(&mut self, new: &NewRequirement) -> Result<bool, RepoError> {
        use crate::schema::requirements::dsl;
        let mut conn = self.get_conn()?;
        let id_val = new
            .id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        diesel::update(dsl::requirements.filter(dsl::id.eq(id_val)))
            .set(new)
            .execute(conn.as_mut())
            .map(|_| true)
            .map_err(|e| e.into())
    }

    fn delete_requirement(&mut self, requirement_id: i32) -> Result<Requirement, RepoError> {
        use crate::schema::requirements::dsl;
        let mut conn = self.get_conn()?;
        let req = dsl::requirements
            .filter(dsl::id.eq(requirement_id))
            .get_result::<Requirement>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(dsl::requirements.filter(dsl::id.eq(requirement_id)))
            .execute(conn.as_mut())?;
        Ok(req)
    }

    fn update_requirement(&mut self, requirement_id: i32) -> Result<(), RepoError> {
        use crate::schema::requirements::dsl;
        use diesel::dsl::now;
        let mut conn = self.get_conn()?;
        diesel::update(dsl::requirements)
            .filter(dsl::id.eq(requirement_id))
            .set(dsl::update_date.eq(now))
            .execute(conn.as_mut())?;
        Ok(())
    }
}

impl TestsCaseRepository for DieselRepo {
    fn get_test_by_id(&self, test_id: i32) -> Result<TestCase, RepoError> {
        use schema::tests::dsl;
        let mut conn = self.get_conn()?;
        dsl::tests
            .filter(dsl::id.eq(test_id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_tests_all(&self) -> Result<Vec<TestCase>, RepoError> {
        use schema::tests::dsl;
        let mut conn = self.get_conn()?;
        dsl::tests
            .order(dsl::id)
            .load::<TestCase>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_tests_by_project(&self, project: i32) -> Result<Vec<TestCase>, RepoError> {
        use schema::tests::dsl;
        let mut conn = self.get_conn()?;
        dsl::tests
            .filter(dsl::project_id.eq(project))
            .load::<TestCase>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_tests_for_requirement(&self, requirement_id: i32) -> Result<Vec<TestCase>, RepoError> {
        use schema::matrix::dsl;
        use schema::tests::dsl as t;
        let mut conn = self.get_conn()?;
        dsl::matrix
            .filter(dsl::req_id.eq(requirement_id))
            .inner_join(t::tests.on(dsl::test_id.eq(t::id)))
            .select((
                t::id,
                t::name,
                t::reference_code,
                t::description,
                t::source,
                t::status_id,
                t::parent_id,
                t::project_id,
            ))
            .load::<TestCase>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_requirements_for_test(&self, test_id: i32) -> Result<Vec<Requirement>, RepoError> {
        use schema::{matrix, requirements};
        let mut conn = self.get_conn()?;
        matrix::dsl::matrix
            .filter(matrix::dsl::test_id.eq(test_id))
            .inner_join(
                requirements::dsl::requirements.on(matrix::dsl::req_id.eq(requirements::dsl::id)),
            )
            .select((
                requirements::dsl::id,
                requirements::dsl::title,
                requirements::dsl::description,
                requirements::dsl::verification_method_id,
                requirements::dsl::status_id,
                requirements::dsl::author_id,
                requirements::dsl::reviewer_id,
                requirements::dsl::reference_code,
                requirements::dsl::category_id,
                requirements::dsl::parent_id,
                requirements::dsl::creation_date,
                requirements::dsl::update_date,
                requirements::dsl::deadline_date,
                requirements::dsl::applicability_id,
                requirements::dsl::justification,
                requirements::dsl::project_id,
            ))
            .load::<Requirement>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn insert_test(&mut self, new: &NewTestCase) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        let res: TestCase = diesel::insert_into(schema::tests::table)
            .values(new)
            .get_result(conn.as_mut())?;
        Ok(res.id)
    }

    fn edit_test(&mut self, new: &NewTestCase) -> Result<bool, RepoError> {
        use crate::schema::tests::dsl;
        let mut conn = self.get_conn()?;
        let test_id_value = new
            .id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let updated = diesel::update(dsl::tests.filter(dsl::id.eq(test_id_value)))
            .set((
                dsl::name.eq(&new.name),
                dsl::description.eq(&new.description),
                dsl::source.eq(&new.source),
                dsl::reference_code.eq(&new.reference_code),
                dsl::status_id.eq(&new.status_id),
                dsl::parent_id.eq(&new.parent_id),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_test(&mut self, test_id: i32) -> Result<TestCase, RepoError> {
        use crate::schema::tests::dsl;
        let mut conn = self.get_conn()?;
        let test = dsl::tests
            .filter(dsl::id.eq(test_id))
            .get_result::<TestCase>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(dsl::tests.filter(dsl::id.eq(test_id))).execute(conn.as_mut())?;
        Ok(test)
    }

    fn update_test_requirement_links(
        &mut self,
        test_id: i32,
        requirement_ids: &[i32],
    ) -> Result<(), RepoError> {
        use schema::matrix::dsl;
        let mut conn = self.get_conn()?;

        conn.as_mut()
            .transaction::<_, diesel::result::Error, _>(|conn| {
                // Delete existing links
                diesel::delete(dsl::matrix.filter(dsl::test_id.eq(test_id))).execute(conn)?;

                // Insert new links
                for requirement_id in requirement_ids {
                    // Get the project_id from the test to maintain consistency
                    use crate::schema::tests::dsl::tests;
                    use crate::schema::tests::dsl::{id as test_id_col, project_id as t_pid};
                    let project_id: i32 = tests
                        .filter(test_id_col.eq(test_id))
                        .select(t_pid)
                        .first(conn)?;

                    let new_matrix = NewMatrixLink {
                        req_id: *requirement_id,
                        test_id: test_id,
                        project_id: project_id,
                    };
                    diesel::insert_into(schema::matrix::table)
                        .values(&new_matrix)
                        .execute(conn)?;
                }
                Ok(())
            })?;
        Ok(())
    }
}

impl crate::repository::LogRepository for DieselRepo {
    fn insert_log(&mut self, new: &NewLog) -> Result<(), RepoError> {
        let mut conn = self.get_conn()?;
        diesel::insert_into(schema::logs::table)
            .values(new)
            .execute(conn.as_mut())?;
        Ok(())
    }

    fn get_logs_recent(&self, limit: i64) -> Result<Vec<Log>, RepoError> {
        use schema::logs::dsl::*;
        let mut conn = self.get_conn()?;
        logs.order(created_at.desc())
            .limit(limit)
            .load::<Log>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_logs_by_entity(&self, etype: &str, eid: i32) -> Result<Vec<Log>, RepoError> {
        use schema::logs::dsl::*;
        let mut conn = self.get_conn()?;
        logs.filter(entity_type.eq(etype))
            .filter(entity_id.eq(eid))
            .order(created_at.desc())
            .load::<Log>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn cleanup_logs(&mut self, days: i64) -> Result<usize, RepoError> {
        use schema::logs::dsl::*;
        let mut conn = self.get_conn()?;
        let cutoff = chrono::Utc::now().naive_utc() - chrono::Duration::days(days);
        let count = diesel::delete(logs.filter(created_at.lt(cutoff))).execute(conn.as_mut())?;
        Ok(count)
    }
}

impl ProjectsRepository for DieselRepo {
    fn get_projects_all(&self) -> Result<Vec<Project>, RepoError> {
        use schema::projects::dsl;
        let mut conn = self.get_conn()?;
        dsl::projects
            .load::<Project>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_project_by_id(&self, project_id: i32) -> Result<Project, RepoError> {
        use schema::projects::dsl;
        let mut conn = self.get_conn()?;
        dsl::projects
            .filter(dsl::id.eq(project_id))
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
        use schema::projects::dsl;
        let mut conn = self.get_conn()?;
        let result = diesel::insert_into(dsl::projects)
            .values(new)
            .get_result::<Project>(conn.as_mut())?;
        Ok(result.id)
    }

    fn edit_project(
        &mut self,
        project_id_param: i32,
        update: &UpdateProject,
    ) -> Result<bool, RepoError> {
        use schema::projects::dsl;
        let mut conn = self.get_conn()?;

        // Build update statement conditionally based on whether status is provided
        let updated = if let Some(status) = update.status {
            diesel::update(dsl::projects.filter(dsl::id.eq(project_id_param)))
                .set((
                    dsl::name.eq(&update.name),
                    dsl::description.eq(&update.description),
                    dsl::status.eq(status),
                    dsl::owner_id.eq(&update.owner_id),
                    dsl::update_date.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(conn.as_mut())?
        } else {
            diesel::update(dsl::projects.filter(dsl::id.eq(project_id_param)))
                .set((
                    dsl::name.eq(&update.name),
                    dsl::description.eq(&update.description),
                    dsl::owner_id.eq(&update.owner_id),
                    dsl::update_date.eq(chrono::Utc::now().naive_utc()),
                ))
                .execute(conn.as_mut())?
        };
        Ok(updated > 0)
    }

    fn delete_project(&mut self, project_id_param: i32) -> Result<Project, RepoError> {
        use schema::projects::dsl;
        let mut conn = self.get_conn()?;
        let proj = dsl::projects
            .filter(dsl::id.eq(project_id_param))
            .get_result::<Project>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(dsl::projects.filter(dsl::id.eq(project_id_param)))
            .execute(conn.as_mut())?;
        Ok(proj)
    }
}

impl MatrixRepository for DieselRepo {
    fn get_matrix_by_project(&self, pid: i32) -> Result<Vec<MatrixLink>, RepoError> {
        use schema::matrix::dsl;
        let mut conn = self.get_conn()?;
        dsl::matrix
            .filter(dsl::project_id.eq(pid))
            .load::<MatrixLink>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn insert_new_matrix_item(&mut self, new: &NewMatrixLink) -> Result<(), RepoError> {
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
}
