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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::sync::{Arc, RwLock};

    fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2023, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn state_with_repo(repo: DieselRepoMock) -> AppState<DieselCachedRepo> {
        AppState {
            repo: Arc::new(RwLock::new(DieselCachedRepo::new(repo, 0))),
        }
    }

    fn actor() -> User {
        DieselRepoMock::make_user(1, "logger", "")
    }

    #[test]
    fn list_all_propagates_connection_error() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let result = service.list_all();
        assert!(matches!(result, Err(RepoError::Pool(_))));
    }

    #[test]
    fn list_by_project_filters_results() {
        let mut repo = DieselRepoMock::default();
        repo.matrices.push(Matrix {
            matrix_req_id: 1,
            matrix_test_id: 10,
            matrix_creation_date: timestamp(),
            project_id: 7,
        });
        repo.matrices.push(Matrix {
            matrix_req_id: 2,
            matrix_test_id: 20,
            matrix_creation_date: timestamp(),
            project_id: 99,
        });

        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        let results = service.list_by_project(7).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].matrix_test_id, 10);
    }

    #[test]
    fn link_inserts_new_matrix_entry() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = MatrixService::new(&state);

        service.link(&actor(), 5, 6, 42).unwrap();

        let entries = service.list_by_project(42).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].matrix_req_id, 5);
        assert_eq!(entries[0].matrix_test_id, 6);
    }
}
