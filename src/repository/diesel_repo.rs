use super::errors::RepoError;
use crate::models::entities::{
    Applicability, Baseline, BaselineTraceability, Category, CustomFieldDefinition,
    CustomFieldValue, CustomFieldValueDisplay, Log, MatrixLink, NewRequirementComment, Project,
    ProjectMember, Requirement, RequirementComment, RequirementContainer, RequirementStatus,
    RequirementVersion, TestCase, TestStatus, User, VerificationMethod,
};
use crate::models::forms::{
    CustomFieldDefinitionPayload, NewApplicability, NewBaselineRequirement, NewBaselineRow,
    NewBaselineTraceability, NewCategory, NewCustomFieldDefinitionRow, NewLog, NewMatrixLink,
    NewProject, NewProjectMember, NewRequirement, NewRequirementContainer, NewRequirementStatus,
    NewTestCase, NewTestStatus, NewUser, NewVerificationMethod, UpdateProject, UpdateUser,
};
use crate::repository::{
    ApiTokensRepository, BaselineRepository, CustomFieldRepository, LookupRepository,
    MatrixRepository, ProjectMembersRepository, ProjectsRepository, RequirementCommentsRepository,
    RequirementsRepository, TestsCaseRepository, UserRepository,
};
use crate::schema;
use diesel::expression_methods::BoolExpressionMethods;
use diesel::expression_methods::NullableExpressionMethods;
use diesel::pg::{upsert::excluded, PgConnection};
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::{Connection, ExpressionMethods, JoinOnDsl, OptionalExtension, QueryDsl, RunQueryDsl};
use std::sync::{Arc, OnceLock};
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
/// Call this from `app::build()` before creating the repository; the pool is stored globally for `DieselRepo::new()`.
pub fn create_connection_pool() -> Result<Arc<ConnectionPool>, Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL must be set")?;
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

/// Initialize the global connection pool. Must be called from `app::build()` before any `DieselRepo::new()`.
pub fn init_connection_pool() -> Result<(), Box<dyn std::error::Error>> {
    CONNECTION_POOL
        .set(create_connection_pool()?)
        .map_err(|_| "connection pool already initialized".into())
}

fn get_pool() -> Result<Arc<ConnectionPool>, Box<dyn std::error::Error>> {
    CONNECTION_POOL.get().cloned().ok_or_else(|| {
        "connection pool not initialized (call init_connection_pool from app::build())".into()
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
    /// initialized (e.g. by the first call to `new()` or by pre-initialization in `app::build()`).
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

impl ApiTokensRepository for DieselRepo {
    fn get_user_by_token_hash(&self, token_hash: &str) -> Result<(User, Option<i32>), RepoError> {
        use schema::user_api_tokens::dsl as tok_dsl;
        let mut conn = self.get_conn()?;
        let row: (User, Option<i32>) = schema::user_api_tokens::table
            .inner_join(
                schema::users::table.on(schema::user_api_tokens::user_id.eq(schema::users::id)),
            )
            .filter(tok_dsl::token_hash.eq(token_hash))
            .select((
                schema::users::all_columns,
                schema::user_api_tokens::project_id,
            ))
            .first(conn.as_mut())
            .map_err(|e: diesel::result::Error| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        Ok(row)
    }

    fn update_api_token_last_used_at(&mut self, token_hash: &str) -> Result<(), RepoError> {
        use schema::user_api_tokens::dsl;
        let mut conn = self.get_conn()?;
        let now = chrono::Utc::now().naive_utc();
        diesel::update(dsl::user_api_tokens.filter(dsl::token_hash.eq(token_hash)))
            .set(dsl::last_used_at.eq(now))
            .execute(conn.as_mut())?;
        Ok(())
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

    fn edit_verification(&mut self, new: &NewVerificationMethod) -> Result<bool, RepoError> {
        use schema::verification::dsl;
        let mut conn = self.get_conn()?;
        let verification_id = new
            .id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let updated = diesel::update(dsl::verification.filter(dsl::id.eq(verification_id)))
            .set((
                dsl::title.eq(&new.title),
                dsl::description.eq(&new.description),
                dsl::tag.eq(&new.tag),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_verification(
        &mut self,
        verification_id: i32,
    ) -> Result<VerificationMethod, RepoError> {
        use schema::verification::dsl;
        let mut conn = self.get_conn()?;
        let verification = dsl::verification
            .filter(dsl::id.eq(verification_id))
            .get_result::<VerificationMethod>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(dsl::verification.filter(dsl::id.eq(verification_id)))
            .execute(conn.as_mut())?;
        Ok(verification)
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

    fn update_requirement_status(
        &mut self,
        id: i32,
        payload: &NewRequirementStatus,
    ) -> Result<bool, RepoError> {
        use schema::requirement_status::dsl;
        let status = self.get_requirement_status_by_id(id)?;
        if status.is_system {
            return Err(RepoError::BadInput("Cannot modify system status".into()));
        }
        let mut conn = self.get_conn()?;
        let updated = diesel::update(dsl::requirement_status.filter(dsl::id.eq(id)))
            .set((
                dsl::title.eq(&payload.title),
                dsl::description.eq(&payload.description),
                dsl::tag.eq(&payload.tag),
                dsl::tag_color.eq(&payload.tag_color),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_requirement_status(&mut self, id: i32) -> Result<RequirementStatus, RepoError> {
        use schema::{requirement_status::dsl, requirement_versions};
        let status = self.get_requirement_status_by_id(id)?;
        if status.is_system {
            return Err(RepoError::BadInput("Cannot delete system status".into()));
        }
        let mut conn = self.get_conn()?;
        let in_use: i64 = requirement_versions::table
            .filter(requirement_versions::status_id.eq(id))
            .count()
            .get_result(conn.as_mut())
            .map_err(RepoError::from)?;
        if in_use > 0 {
            return Err(RepoError::BadInput(
                "Cannot delete status: it is in use by requirement versions".into(),
            ));
        }
        diesel::delete(dsl::requirement_status.filter(dsl::id.eq(id))).execute(conn.as_mut())?;
        Ok(status)
    }

    fn update_test_status(&mut self, id: i32, payload: &NewTestStatus) -> Result<bool, RepoError> {
        use schema::test_status::dsl;
        let status = self.get_test_status_by_id(id)?;
        if status.is_system {
            return Err(RepoError::BadInput("Cannot modify system status".into()));
        }
        let mut conn = self.get_conn()?;
        let updated = diesel::update(dsl::test_status.filter(dsl::id.eq(id)))
            .set((
                dsl::title.eq(&payload.title),
                dsl::description.eq(&payload.description),
                dsl::tag.eq(&payload.tag),
                dsl::tag_color.eq(&payload.tag_color),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_test_status(&mut self, id: i32) -> Result<TestStatus, RepoError> {
        use schema::{test_status::dsl, tests};
        let status = self.get_test_status_by_id(id)?;
        if status.is_system {
            return Err(RepoError::BadInput("Cannot delete system status".into()));
        }
        let mut conn = self.get_conn()?;
        let in_use: i64 = tests::table
            .filter(tests::status_id.eq(id))
            .count()
            .get_result(conn.as_mut())
            .map_err(RepoError::from)?;
        if in_use > 0 {
            return Err(RepoError::BadInput(
                "Cannot delete status: it is in use by tests".into(),
            ));
        }
        diesel::delete(dsl::test_status.filter(dsl::id.eq(id))).execute(conn.as_mut())?;
        Ok(status)
    }
}

/// Builds a Requirement for baseline context using the snapshot version id (so links point to the version).
fn requirement_from_baseline_version(
    container: &RequirementContainer,
    version: &RequirementVersion,
) -> Requirement {
    let same_as_current = container.current_version_id == Some(version.id);
    Requirement {
        id: container.id,
        current_version_id: Some(version.id),
        same_as_current: Some(same_as_current),
        title: version.title.clone(),
        description: version.description.clone(),
        status_id: version.status_id,
        author_id: version.author_id,
        reviewer_id: version.reviewer_id,
        reference_code: container.stable_code.clone(),
        category_id: version.category_id,
        parent_id: version.parent_id,
        creation_date: version.created_at,
        update_date: version.created_at,
        deadline_date: version.deadline_date,
        applicability_id: version.applicability_id,
        justification: version.justification.clone(),
        project_id: container.project_id,
        approval_state: version.approval_state.clone(),
        approved_by: version.approved_by,
        approved_at: version.approved_at,
        custom_fields: None,
    }
}

fn requirement_from_current(
    container: &RequirementContainer,
    version: &RequirementVersion,
) -> Requirement {
    Requirement {
        id: container.id,
        current_version_id: container.current_version_id,
        same_as_current: None,
        title: version.title.clone(),
        description: version.description.clone(),
        status_id: version.status_id,
        author_id: version.author_id,
        reviewer_id: version.reviewer_id,
        reference_code: container.stable_code.clone(),
        category_id: version.category_id,
        parent_id: version.parent_id,
        creation_date: version.created_at,
        update_date: version.created_at,
        deadline_date: version.deadline_date,
        applicability_id: version.applicability_id,
        justification: version.justification.clone(),
        project_id: container.project_id,
        approval_state: version.approval_state.clone(),
        approved_by: version.approved_by,
        approved_at: version.approved_at,
        custom_fields: None,
    }
}

impl RequirementsRepository for DieselRepo {
    fn get_requirement_by_id(&self, requirement_id: i32) -> Result<Requirement, RepoError> {
        use schema::requirement_versions;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        let (container, version): (RequirementContainer, RequirementVersion) = requirements::table
            .inner_join(
                requirement_versions::table
                    .on(requirements::current_version_id.eq(requirement_versions::id.nullable())),
            )
            .filter(requirements::id.eq(requirement_id))
            .select((requirements::all_columns, requirement_versions::all_columns))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        Ok(requirement_from_current(&container, &version))
    }

    fn get_requirements_all(&self) -> Result<Vec<Requirement>, RepoError> {
        use schema::requirement_versions;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        let rows: Vec<(RequirementContainer, RequirementVersion)> = requirements::table
            .inner_join(
                requirement_versions::table
                    .on(requirements::current_version_id.eq(requirement_versions::id.nullable())),
            )
            .order(requirements::id)
            .select((requirements::all_columns, requirement_versions::all_columns))
            .load(conn.as_mut())
            .map_err(RepoError::from)?;
        Ok(rows
            .into_iter()
            .map(|(c, v)| requirement_from_current(&c, &v))
            .collect())
    }

    fn get_requirements_by_project(
        &self,
        project_id_param: i32,
    ) -> Result<Vec<Requirement>, RepoError> {
        use schema::requirement_versions;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        let rows: Vec<(RequirementContainer, RequirementVersion)> = requirements::table
            .inner_join(
                requirement_versions::table
                    .on(requirements::current_version_id.eq(requirement_versions::id.nullable())),
            )
            .filter(requirements::project_id.eq(project_id_param))
            .order(requirements::id)
            .select((requirements::all_columns, requirement_versions::all_columns))
            .load(conn.as_mut())
            .map_err(RepoError::from)?;
        Ok(rows
            .into_iter()
            .map(|(c, v)| requirement_from_current(&c, &v))
            .collect())
    }

    fn get_requirements_by_project_filtered_paginated(
        &self,
        project_id: i32,
        status_filter: Option<i32>,
        verification_filter: Option<i32>,
        category_filter: Option<i32>,
        applicability_filter: Option<i32>,
        custom_field_filters: Option<&[(i32, String)]>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Requirement>, RepoError> {
        use schema::custom_field_values::dsl as cfv_dsl;
        use schema::requirement_version_verification_methods::dsl as rvvm_dsl;
        use schema::requirement_versions;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        let mut query = requirements::table
            .inner_join(
                requirement_versions::table
                    .on(requirements::current_version_id.eq(requirement_versions::id.nullable())),
            )
            .filter(requirements::project_id.eq(project_id))
            .into_boxed();
        if let Some(s) = status_filter {
            query = query.filter(requirement_versions::status_id.eq(s));
        }
        if let Some(c) = category_filter {
            query = query.filter(requirement_versions::category_id.eq(c));
        }
        if let Some(a) = applicability_filter {
            query = query.filter(requirement_versions::applicability_id.eq(a));
        }
        if let Some(v) = verification_filter {
            query = query.filter(
                requirement_versions::id.eq_any(
                    rvvm_dsl::requirement_version_verification_methods
                        .filter(rvvm_dsl::verification_method_id.eq(v))
                        .select(rvvm_dsl::requirement_version_id),
                ),
            );
        }
        if let Some(filters) = custom_field_filters {
            for (field_id, value) in filters.iter() {
                let version_ids = cfv_dsl::custom_field_values
                    .filter(cfv_dsl::custom_field_definition_id.eq(field_id))
                    .filter(cfv_dsl::value.eq(value));
                query = query.filter(
                    requirement_versions::id
                        .eq_any(version_ids.select(cfv_dsl::requirement_version_id)),
                );
            }
        }
        let rows: Vec<(RequirementContainer, RequirementVersion)> = query
            .order(requirements::id)
            .limit(limit)
            .offset(offset)
            .select((requirements::all_columns, requirement_versions::all_columns))
            .load(conn.as_mut())
            .map_err(RepoError::from)?;
        // Sort empty stable_code last (same as legacy)
        let mut result: Vec<Requirement> = rows
            .into_iter()
            .map(|(c, v)| requirement_from_current(&c, &v))
            .collect();
        result.sort_by(|a, b| {
            match (
                a.reference_code.trim().is_empty(),
                b.reference_code.trim().is_empty(),
            ) {
                (false, false) => a.reference_code.cmp(&b.reference_code),
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                (true, true) => a.id.cmp(&b.id),
            }
        });
        Ok(result)
    }

    fn insert_new_requirement(&mut self, new: &NewRequirement) -> Result<i32, RepoError> {
        use schema::requirement_versions;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        conn.as_mut().transaction::<i32, RepoError, _>(|conn| {
            let container = NewRequirementContainer {
                project_id: new.project_id,
                stable_code: new.reference_code.clone(),
                current_version_id: None,
            };
            let req_id: i32 = diesel::insert_into(requirements::table)
                .values(&container)
                .returning(requirements::id)
                .get_result(conn)?;
            let version = new.to_new_version(req_id);
            let version_id: i32 = diesel::insert_into(requirement_versions::table)
                .values(&version)
                .returning(requirement_versions::id)
                .get_result(conn)?;
            diesel::update(requirements::table.filter(requirements::id.eq(req_id)))
                .set(requirements::current_version_id.eq(version_id))
                .execute(conn)?;
            Ok(req_id)
        })
    }

    fn get_verification_method_ids_for_requirement(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<i32>, RepoError> {
        use schema::requirement_version_verification_methods::dsl as rvvm_dsl;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        let current_version_id: Option<i32> = requirements::table
            .filter(requirements::id.eq(requirement_id))
            .select(requirements::current_version_id)
            .get_result::<Option<i32>>(conn.as_mut())
            .optional()
            .map_err(RepoError::from)?
            .flatten();
        let Some(vid) = current_version_id else {
            return Ok(vec![]);
        };
        rvvm_dsl::requirement_version_verification_methods
            .filter(rvvm_dsl::requirement_version_id.eq(vid))
            .select(rvvm_dsl::verification_method_id)
            .order(rvvm_dsl::verification_method_id)
            .load::<i32>(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn get_verification_method_ids_for_version(
        &self,
        version_id: i32,
    ) -> Result<Vec<i32>, RepoError> {
        use schema::requirement_version_verification_methods::dsl as rvvm_dsl;
        let mut conn = self.get_conn()?;
        rvvm_dsl::requirement_version_verification_methods
            .filter(rvvm_dsl::requirement_version_id.eq(version_id))
            .select(rvvm_dsl::verification_method_id)
            .order(rvvm_dsl::verification_method_id)
            .load::<i32>(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn get_requirement_ids_by_verification_method(
        &self,
        verification_method_id: i32,
    ) -> Result<Vec<i32>, RepoError> {
        use schema::requirement_version_verification_methods::dsl as rvvm_dsl;
        use schema::requirement_versions;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        requirements::table
            .inner_join(
                requirement_versions::table
                    .on(requirements::current_version_id.eq(requirement_versions::id.nullable())),
            )
            .inner_join(
                rvvm_dsl::requirement_version_verification_methods
                    .on(rvvm_dsl::requirement_version_id.eq(requirement_versions::id)),
            )
            .filter(rvvm_dsl::verification_method_id.eq(verification_method_id))
            .select(requirements::id)
            .load::<i32>(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn set_requirement_verification_methods(
        &mut self,
        requirement_id: i32,
        verification_method_ids: &[i32],
    ) -> Result<(), RepoError> {
        use schema::requirement_version_verification_methods;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        let current_version_id: Option<i32> = requirements::table
            .filter(requirements::id.eq(requirement_id))
            .select(requirements::current_version_id)
            .get_result::<Option<i32>>(conn.as_mut())
            .optional()
            .map_err(RepoError::from)?
            .flatten();
        let Some(vid) = current_version_id else {
            return Ok(());
        };
        diesel::delete(requirement_version_verification_methods::table)
            .filter(requirement_version_verification_methods::requirement_version_id.eq(vid))
            .execute(conn.as_mut())?;
        for &verification_method_id in verification_method_ids {
            if verification_method_id <= 0 {
                continue;
            }
            diesel::insert_into(requirement_version_verification_methods::table)
                .values((
                    requirement_version_verification_methods::requirement_version_id.eq(vid),
                    requirement_version_verification_methods::verification_method_id
                        .eq(verification_method_id),
                ))
                .execute(conn.as_mut())?;
        }
        Ok(())
    }

    fn edit_requirement(&mut self, new: &NewRequirement) -> Result<bool, RepoError> {
        use schema::requirement_versions;
        use schema::requirements;
        let id_val = new
            .id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let mut conn = self.get_conn()?;
        conn.as_mut().transaction::<bool, RepoError, _>(|conn| {
            let current_version_id: Option<i32> = requirements::table
                .filter(requirements::id.eq(id_val))
                .select(requirements::current_version_id)
                .get_result::<Option<i32>>(conn)
                .optional()?
                .flatten();
            let Some(_) = current_version_id else {
                return Err(diesel::result::Error::NotFound.into());
            };
            let version = new.to_new_version(id_val);
            let new_version_id: i32 = diesel::insert_into(requirement_versions::table)
                .values(&version)
                .returning(requirement_versions::id)
                .get_result(conn)?;
            let affected = diesel::update(requirements::table.filter(requirements::id.eq(id_val)))
                .set(requirements::current_version_id.eq(new_version_id))
                .execute(conn)?;
            Ok(affected > 0)
        })
    }

    fn delete_requirement(&mut self, requirement_id: i32) -> Result<Requirement, RepoError> {
        let req = self.get_requirement_by_id(requirement_id)?;
        let mut conn = self.get_conn()?;
        diesel::delete(
            schema::requirements::table.filter(schema::requirements::id.eq(requirement_id)),
        )
        .execute(conn.as_mut())?;
        Ok(req)
    }

    fn update_requirement(&mut self, _requirement_id: i32) -> Result<(), RepoError> {
        // Versions are immutable; no update_date to touch. No-op for compatibility.
        Ok(())
    }

    fn list_requirement_versions(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<RequirementVersion>, RepoError> {
        use schema::requirement_versions::dsl;
        let mut conn = self.get_conn()?;
        dsl::requirement_versions
            .filter(dsl::requirement_id.eq(requirement_id))
            .order(dsl::created_at.desc())
            .load(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn get_requirement_version_by_id(
        &self,
        version_id: i32,
    ) -> Result<RequirementVersion, RepoError> {
        use schema::requirement_versions::dsl;
        let mut conn = self.get_conn()?;
        dsl::requirement_versions
            .filter(dsl::id.eq(version_id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn set_requirement_version_approval(
        &mut self,
        version_id: i32,
        new_state: &str,
        approved_by_user_id: i32,
    ) -> Result<RequirementVersion, RepoError> {
        use crate::status_enums::ApprovalState;
        use schema::requirement_versions::dsl;
        let mut conn = self.get_conn()?;
        let version: RequirementVersion = dsl::requirement_versions
            .filter(dsl::id.eq(version_id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        let current = ApprovalState::from_db_string(&version.approval_state).ok_or_else(|| {
            RepoError::BadInput(format!(
                "invalid approval_state in DB: {}",
                version.approval_state
            ))
        })?;
        let target = ApprovalState::from_db_string(new_state)
            .ok_or_else(|| RepoError::BadInput(format!("invalid approval_state: {}", new_state)))?;
        if !current.can_transition_to(target) {
            return Err(RepoError::BadInput(format!(
                "invalid transition: {} -> {}",
                version.approval_state, new_state
            )));
        }
        // Idempotent: already in target state — return version unchanged
        if current == target {
            return Ok(version);
        }
        let now = chrono::Utc::now().naive_utc();
        let (approved_by, approved_at) = if target == ApprovalState::Approved {
            (Some(approved_by_user_id), Some(now))
        } else {
            (version.approved_by, version.approved_at)
        };
        diesel::update(dsl::requirement_versions.filter(dsl::id.eq(version_id)))
            .set((
                dsl::approval_state.eq(target.to_db_string()),
                dsl::approved_by.eq(approved_by),
                dsl::approved_at.eq(approved_at),
            ))
            .execute(conn.as_mut())?;
        drop(conn);
        let _ = self.mark_links_suspect_for_requirement(
            version.requirement_id,
            "Approval state changed",
            Some(version_id),
            Some(approved_by_user_id),
        )?;
        let mut conn = self.get_conn()?;
        dsl::requirement_versions
            .filter(dsl::id.eq(version_id))
            .get_result(conn.as_mut())
            .map_err(RepoError::from)
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

    fn get_impacted_tests_for_requirement(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<TestCase>, RepoError> {
        use schema::matrix::dsl;
        use schema::tests::dsl as t;
        let mut conn = self.get_conn()?;
        dsl::matrix
            .filter(dsl::req_id.eq(requirement_id))
            .filter(dsl::suspect.eq(true))
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
        use schema::matrix;
        use schema::requirement_versions;
        use schema::requirements;
        let mut conn = self.get_conn()?;
        let rows: Vec<(RequirementContainer, RequirementVersion)> = matrix::table
            .filter(matrix::test_id.eq(test_id))
            .inner_join(requirements::table.on(matrix::req_id.eq(requirements::id)))
            .inner_join(
                requirement_versions::table
                    .on(requirements::current_version_id.eq(requirement_versions::id.nullable())),
            )
            .select((requirements::all_columns, requirement_versions::all_columns))
            .load(conn.as_mut())
            .map_err(RepoError::from)?;
        Ok(rows
            .into_iter()
            .map(|(c, v)| requirement_from_current(&c, &v))
            .collect())
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
                        test_id,
                        project_id,
                        triggering_version_id: None,
                        triggering_user_id: None,
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

impl RequirementCommentsRepository for DieselRepo {
    fn insert_requirement_comment(
        &mut self,
        new: &NewRequirementComment,
    ) -> Result<RequirementComment, RepoError> {
        let mut conn = self.get_conn()?;
        diesel::insert_into(schema::requirement_comments::table)
            .values(new)
            .returning(schema::requirement_comments::all_columns)
            .get_result(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn list_comments_by_requirement(
        &self,
        requirement_id: i32,
        version_id: Option<i32>,
    ) -> Result<Vec<RequirementComment>, RepoError> {
        use schema::requirement_comments::dsl;
        let mut conn = self.get_conn()?;
        let q = dsl::requirement_comments
            .filter(dsl::requirement_id.eq(requirement_id))
            .order(dsl::created_at.asc());
        let rows = match version_id {
            Some(vid) => q
                .filter(
                    dsl::requirement_version_id
                        .is_null()
                        .or(dsl::requirement_version_id.eq(vid)),
                )
                .load::<RequirementComment>(conn.as_mut()),
            None => q.load::<RequirementComment>(conn.as_mut()),
        };
        rows.map_err(RepoError::from)
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

    fn mark_links_suspect_for_requirement(
        &mut self,
        requirement_id: i32,
        reason: &str,
        triggering_version_id: Option<i32>,
        triggering_user_id: Option<i32>,
    ) -> Result<Vec<i32>, RepoError> {
        use schema::matrix::dsl;
        let now = chrono::Utc::now().naive_utc();
        let mut conn = self.get_conn()?;
        let updated: Vec<i32> = diesel::update(dsl::matrix.filter(dsl::req_id.eq(requirement_id)))
            .set((
                dsl::suspect.eq(true),
                dsl::suspect_at.eq(now),
                dsl::suspect_reason.eq(reason),
                dsl::cleared_by.eq(Option::<i32>::None),
                dsl::cleared_at.eq(Option::<chrono::NaiveDateTime>::None),
                dsl::triggering_version_id.eq(triggering_version_id),
                dsl::triggering_user_id.eq(triggering_user_id),
            ))
            .returning(dsl::project_id)
            .get_results(conn.as_mut())?;
        Ok(updated.into_iter().collect())
    }

    fn clear_suspect(
        &mut self,
        req_id: i32,
        test_id: i32,
        cleared_by_user_id: i32,
    ) -> Result<(bool, Option<i32>), RepoError> {
        use schema::matrix::dsl;
        let now = chrono::Utc::now().naive_utc();
        let mut conn = self.get_conn()?;
        let project_id: Option<i32> = diesel::update(
            dsl::matrix
                .filter(dsl::req_id.eq(req_id))
                .filter(dsl::test_id.eq(test_id)),
        )
        .set((
            dsl::suspect.eq(false),
            dsl::suspect_at.eq(Option::<chrono::NaiveDateTime>::None),
            dsl::suspect_reason.eq(Option::<String>::None),
            dsl::cleared_by.eq(cleared_by_user_id),
            dsl::cleared_at.eq(now),
        ))
        .returning(dsl::project_id)
        .get_result(conn.as_mut())
        .optional()?;
        Ok((project_id.is_some(), project_id))
    }
}

impl CustomFieldRepository for DieselRepo {
    fn list_custom_field_definitions_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<CustomFieldDefinition>, RepoError> {
        use schema::custom_field_definitions::dsl;
        let mut conn = self.get_conn()?;
        dsl::custom_field_definitions
            .filter(dsl::project_id.eq(project_id))
            .order((dsl::sort_order, dsl::id))
            .load(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn get_custom_field_definition_by_id(
        &self,
        id: i32,
    ) -> Result<CustomFieldDefinition, RepoError> {
        use schema::custom_field_definitions::dsl;
        let mut conn = self.get_conn()?;
        dsl::custom_field_definitions
            .filter(dsl::id.eq(id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn create_custom_field_definition(
        &mut self,
        project_id: i32,
        payload: &CustomFieldDefinitionPayload,
    ) -> Result<i32, RepoError> {
        let enum_values_json = payload
            .enum_values
            .as_ref()
            .map(|v| serde_json::to_value(v).map_err(|e| RepoError::BadInput(e.to_string())))
            .transpose()?;
        let row = NewCustomFieldDefinitionRow {
            project_id,
            label: payload.label.trim().to_string(),
            field_type: payload.field_type.trim().to_lowercase(),
            enum_values: enum_values_json,
            sort_order: payload.sort_order.unwrap_or(0),
        };
        validate_custom_field_payload(&row.field_type, row.enum_values.as_ref())?;
        use schema::custom_field_definitions::dsl;
        let mut conn = self.get_conn()?;
        diesel::insert_into(dsl::custom_field_definitions)
            .values(&row)
            .returning(dsl::id)
            .get_result(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn update_custom_field_definition(
        &mut self,
        id: i32,
        payload: &CustomFieldDefinitionPayload,
    ) -> Result<(), RepoError> {
        let enum_values_json = payload
            .enum_values
            .as_ref()
            .map(|v| serde_json::to_value(v).map_err(|e| RepoError::BadInput(e.to_string())))
            .transpose()?;
        let field_type = payload.field_type.trim().to_lowercase();
        validate_custom_field_payload(&field_type, enum_values_json.as_ref())?;
        use schema::custom_field_definitions::dsl;
        let mut conn = self.get_conn()?;
        let affected = diesel::update(dsl::custom_field_definitions.filter(dsl::id.eq(id)))
            .set((
                dsl::label.eq(payload.label.trim()),
                dsl::field_type.eq(&field_type),
                dsl::enum_values.eq(enum_values_json),
                dsl::sort_order.eq(payload.sort_order.unwrap_or(0)),
            ))
            .execute(conn.as_mut())?;
        if affected == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }

    fn count_requirement_versions_using_field(&self, field_id: i32) -> Result<i64, RepoError> {
        use schema::custom_field_values::dsl;
        let mut conn = self.get_conn()?;
        dsl::custom_field_values
            .filter(dsl::custom_field_definition_id.eq(field_id))
            .count()
            .get_result(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn delete_custom_field_definition(&mut self, id: i32) -> Result<(), RepoError> {
        use schema::custom_field_definitions::dsl;
        let mut conn = self.get_conn()?;
        let affected = diesel::delete(dsl::custom_field_definitions.filter(dsl::id.eq(id)))
            .execute(conn.as_mut())?;
        if affected == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }

    fn get_custom_field_values_for_version(
        &self,
        version_id: i32,
    ) -> Result<Vec<CustomFieldValueDisplay>, RepoError> {
        use schema::custom_field_definitions;
        use schema::custom_field_values;
        let mut conn = self.get_conn()?;
        let rows: Vec<(CustomFieldValue, CustomFieldDefinition)> = custom_field_values::table
            .inner_join(custom_field_definitions::table.on(
                custom_field_values::custom_field_definition_id.eq(custom_field_definitions::id),
            ))
            .filter(custom_field_values::requirement_version_id.eq(version_id))
            .select((
                custom_field_values::all_columns,
                custom_field_definitions::all_columns,
            ))
            .load(conn.as_mut())
            .map_err(RepoError::from)?;
        Ok(rows
            .into_iter()
            .map(|(v, d)| CustomFieldValueDisplay {
                field_id: d.id,
                label: d.label,
                value: v.value,
            })
            .collect())
    }

    fn set_custom_field_values_for_version(
        &mut self,
        version_id: i32,
        values: &[(i32, Option<String>)],
    ) -> Result<(), RepoError> {
        use schema::custom_field_values::dsl;
        let mut conn = self.get_conn()?;
        diesel::delete(dsl::custom_field_values.filter(dsl::requirement_version_id.eq(version_id)))
            .execute(conn.as_mut())?;
        for &(field_id, ref value) in values {
            if field_id <= 0 {
                continue;
            }
            diesel::insert_into(dsl::custom_field_values)
                .values((
                    dsl::requirement_version_id.eq(version_id),
                    dsl::custom_field_definition_id.eq(field_id),
                    dsl::value.eq(value.as_deref()),
                ))
                .execute(conn.as_mut())?;
        }
        Ok(())
    }
}

fn validate_custom_field_payload(
    field_type: &str,
    enum_values: Option<&serde_json::Value>,
) -> Result<(), RepoError> {
    const VALID_TYPES: &[&str] = &["text", "enum", "boolean", "number"];
    if !VALID_TYPES.contains(&field_type) {
        return Err(RepoError::BadInput(format!(
            "field_type must be one of: {}",
            VALID_TYPES.join(", ")
        )));
    }
    if field_type == "enum" {
        let arr = enum_values.and_then(|v| v.as_array()).ok_or_else(|| {
            RepoError::BadInput("enum field_type requires enum_values array".into())
        })?;
        if arr.is_empty() {
            return Err(RepoError::BadInput(
                "enum field_type requires at least one enum value".into(),
            ));
        }
    }
    Ok(())
}

impl BaselineRepository for DieselRepo {
    fn create_baseline(
        &mut self,
        project_id: i32,
        created_by: i32,
        payload: &crate::models::NewBaseline,
    ) -> Result<Baseline, RepoError> {
        use schema::baseline_requirements;
        use schema::baseline_traceability;
        use schema::baselines;
        use schema::matrix;
        use schema::requirement_versions;
        use schema::requirements;

        let mut conn = self.get_conn()?;
        conn.as_mut().transaction::<_, RepoError, _>(|conn| {
            let now = chrono::Utc::now().naive_utc();
            let new_row = NewBaselineRow {
                project_id,
                name: payload.name.clone(),
                description: payload.description.clone(),
                created_at: now,
                created_by,
            };
            let baseline: Baseline = diesel::insert_into(baselines::table)
                .values(&new_row)
                .get_result(conn)?;
            let baseline_id = baseline.id;

            // Snapshot: all requirements in project with their current version (point-in-time)
            let rows: Vec<(RequirementContainer, RequirementVersion)> =
                requirements::table
                    .inner_join(requirement_versions::table.on(
                        requirements::current_version_id.eq(requirement_versions::id.nullable()),
                    ))
                    .filter(requirements::project_id.eq(project_id))
                    .select((requirements::all_columns, requirement_versions::all_columns))
                    .load(conn)?;
            for (container, version) in rows {
                let br = NewBaselineRequirement {
                    baseline_id,
                    requirement_id: container.id,
                    version_id: version.id,
                };
                diesel::insert_into(baseline_requirements::table)
                    .values(&br)
                    .execute(conn)?;
            }

            // Snapshot: current traceability matrix (including suspect state at baseline time)
            let matrix_links: Vec<MatrixLink> = matrix::table
                .filter(matrix::project_id.eq(project_id))
                .load(conn)?;
            for link in matrix_links {
                let bt = NewBaselineTraceability {
                    baseline_id,
                    requirement_id: link.req_id,
                    test_id: link.test_id,
                    suspect: link.suspect,
                    suspect_at: link.suspect_at,
                    suspect_reason: link.suspect_reason.clone(),
                };
                diesel::insert_into(baseline_traceability::table)
                    .values(&bt)
                    .execute(conn)?;
            }

            Ok(baseline)
        })
    }

    fn list_baselines_by_project(&self, project_id: i32) -> Result<Vec<Baseline>, RepoError> {
        use schema::baselines::dsl;
        let mut conn = self.get_conn()?;
        dsl::baselines
            .filter(dsl::project_id.eq(project_id))
            .order(dsl::created_at.desc())
            .load(conn.as_mut())
            .map_err(RepoError::from)
    }

    fn get_baseline_by_id(&self, baseline_id: i32) -> Result<Baseline, RepoError> {
        use schema::baselines::dsl;
        let mut conn = self.get_conn()?;
        dsl::baselines
            .filter(dsl::id.eq(baseline_id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_requirements_for_baseline(
        &self,
        baseline_id: i32,
    ) -> Result<Vec<Requirement>, RepoError> {
        use schema::baseline_requirements;
        use schema::requirement_versions;
        use schema::requirements;

        let mut conn = self.get_conn()?;
        let rows: Vec<(RequirementContainer, RequirementVersion)> = baseline_requirements::table
            .inner_join(
                requirement_versions::table
                    .on(baseline_requirements::version_id.eq(requirement_versions::id)),
            )
            .inner_join(
                requirements::table.on(baseline_requirements::requirement_id.eq(requirements::id)),
            )
            .filter(baseline_requirements::baseline_id.eq(baseline_id))
            .select((requirements::all_columns, requirement_versions::all_columns))
            .load(conn.as_mut())?;
        Ok(rows
            .into_iter()
            .map(|(c, v)| requirement_from_baseline_version(&c, &v))
            .collect())
    }

    fn get_baseline_requirement_version_id(
        &self,
        baseline_id: i32,
        requirement_id: i32,
    ) -> Result<Option<i32>, RepoError> {
        use schema::baseline_requirements::dsl;
        let mut conn = self.get_conn()?;
        dsl::baseline_requirements
            .filter(dsl::baseline_id.eq(baseline_id))
            .filter(dsl::requirement_id.eq(requirement_id))
            .select(dsl::version_id)
            .get_result::<i32>(conn.as_mut())
            .optional()
            .map_err(RepoError::from)
    }

    fn get_baseline_traceability(
        &self,
        baseline_id: i32,
    ) -> Result<Vec<BaselineTraceability>, RepoError> {
        use schema::baseline_traceability::dsl;
        let mut conn = self.get_conn()?;
        dsl::baseline_traceability
            .filter(dsl::baseline_id.eq(baseline_id))
            .order((dsl::requirement_id.asc(), dsl::test_id.asc()))
            .load(conn.as_mut())
            .map_err(RepoError::from)
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
