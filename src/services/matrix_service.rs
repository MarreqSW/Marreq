//! Service providing traceability matrix operations.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{ActionType, EntityType, Matrix, NewMatrix, User};
use crate::repository::errors::RepoError;
use crate::repository::{MatrixRepository, PooledConnectionWrapper};
use diesel::prelude::*;

/// High level matrix operations backed by the shared [`AppState`].
pub struct MatrixService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> MatrixService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve every matrix entry.
    pub fn list_all(&self) -> Result<Vec<Matrix>, RepoError> {
        use crate::schema::matrix::dsl::matrix;

        let mut conn = self.db_connection()?;
        matrix
            .load::<Matrix>(conn.as_mut())
            .map_err(RepoError::from)
    }

    /// Retrieve matrix entries scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<Matrix>, RepoError> {
        self.state.repo_read().get_matrix_by_project(project_id)
    }

    /// Create a new traceability link between a requirement and a test.
    pub fn link(
        &self,
        actor: &User,
        requirement_id: i32,
        test_id: i32,
        project_id: i32,
    ) -> Result<(), RepoError> {
        let payload = NewMatrix {
            matrix_req_id: requirement_id,
            matrix_test_id: test_id,
            project_id,
        };

        {
            let mut repo = self.state.repo_write();
            repo.insert_new_matrix_item(&payload)?;
        }

        self.log_link_created(actor, &payload);
        Ok(())
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn log_link_created(&self, actor: &User, entity: &NewMatrix) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(actor.user_id);
            let description = format!(
                "Linked requirement {} with test {}",
                entity.matrix_req_id, entity.matrix_test_id
            );

            if let Err(_err) = Logger::log_custom(
                conn.as_mut(),
                &ctx,
                ActionType::Create,
                EntityType::Matrix,
                None,
                Some(entity.project_id),
                None,
                None,
                Some(description),
            ) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log matrix link {} -> {}: {_err}",
                    entity.matrix_req_id, entity.matrix_test_id
                );
            }
        }
    }
}
