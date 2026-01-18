//! Category service for managing requirement categories business logic.

use crate::app::{AppState, DieselCachedRepo};
use crate::logger::{LogCtx, Logger};
use crate::models::{Category, NewCategory, User};
use crate::repository::errors::RepoError;
use crate::repository::{LookupRepository, PooledConnectionWrapper};

pub struct CategoryService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> CategoryService<'a> {
    /// Create a new service instance bound to the provided application state.
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    /// Retrieve all category entries.
    pub fn list_all(&self) -> Result<Vec<Category>, RepoError> {
        self.state.repo_read().get_categories_all()
    }

    /// Retrieve Category entries scoped to a project.
    pub fn list_by_project(&self, project_id: i32) -> Result<Vec<Category>, RepoError> {
        self.state.repo_read().get_categories_by_project(project_id)
    }

    /// Retrieve a single Category by identifier.
    pub fn get_by_id(&self, id: i32) -> Result<Category, RepoError> {
        self.state.repo_read().get_category_by_id(id)
    }

    pub fn get_category_name(&self, id: i32) -> Result<String, RepoError> {
        let category = self.state.repo_read().get_category_by_id(id)?;
        Ok(category.title)
    }

    /// Create a new Category entry and log the action.
    pub fn create(&self, user: &User, new_cat: NewCategory) -> Result<i32, RepoError> {
        let id = {
            let mut repo = self.state.repo_write();
            repo.insert_new_category(&new_cat)?
        };

        self.log_created(user, id, &new_cat);
        Ok(id)
    }

    /// Update an existing Category entry and log the change.
    pub fn update(
        &self,
        user: &User,
        id: i32,
        mut updated_cat: NewCategory,
    ) -> Result<Category, RepoError> {
        let before = self.get_by_id(id)?;

        updated_cat.id = Some(id);

        {
            let mut repo = self.state.repo_write();
            let updated = repo.edit_category(&updated_cat)?;
            if !updated {
                return Err(RepoError::NotFound);
            }
        }

        let after = self.get_by_id(id)?;
        self.log_updated(user, &before, &after);
        Ok(after)
    }

    /// Delete an Category entry and log the removal.
    pub fn delete(&self, user: &User, id: i32) -> Result<Category, RepoError> {
        let deleted = {
            let mut repo = self.state.repo_write();
            repo.delete_category(id)?
        };

        self.log_deleted(user, &deleted);
        Ok(deleted)
    }

    fn db_connection(&self) -> Result<PooledConnectionWrapper, RepoError> {
        self.state.repo_read().inner_repo().get_conn()
    }

    fn log_created(&self, user: &User, id: i32, entity: &NewCategory) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.id);
            if let Err(_err) = Logger::created(conn.as_mut(), &ctx, id, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log category creation {id}: {_err}");
            }
        }
    }

    fn log_updated(&self, user: &User, before: &Category, after: &Category) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.id);
            if let Err(_err) = Logger::updated(conn.as_mut(), &ctx, before, after) {
                #[cfg(debug_assertions)]
                eprintln!(
                    "Failed to log category update {} -> {}: {_err}",
                    before.id, after.id
                );
            }
        }
    }

    fn log_deleted(&self, user: &User, entity: &Category) {
        if let Ok(mut conn) = self.db_connection() {
            let ctx = LogCtx::new(user.id);
            if let Err(_err) = Logger::deleted(conn.as_mut(), &ctx, entity) {
                #[cfg(debug_assertions)]
                eprintln!("Failed to log category deletion {}: {_err}", entity.id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use std::sync::{Arc, RwLock};

    fn state_with_repo(repo: DieselRepoMock) -> AppState<DieselCachedRepo> {
        AppState {
            repo: Arc::new(RwLock::new(DieselCachedRepo::new(repo, 0))),
        }
    }

    fn actor() -> User {
        DieselRepoMock::make_user(1, "actor", "")
    }

    fn category(id: i32, title: &str, project_id: i32) -> Category {
        Category {
            id: id,
            title: title.into(),
            description: "desc".into(),
            tag: "TAG".into(),
            project_id,
        }
    }

    #[test]
    fn create_inserts_new_category() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = CategoryService::new(&state);

        let payload = NewCategory {
            id: None,
            title: "Primary".into(),
            description: "Main".into(),
            tag: "MAIN".into(),
            project_id: 2,
        };

        let id = service.create(&actor(), payload).unwrap();
        let stored = service.get_by_id(id).unwrap();
        assert_eq!(stored.title, "Primary");
    }

    #[test]
    fn update_modifies_existing_category() {
        let mut repo = DieselRepoMock::default();
        repo.categories.insert(1, category(1, "Legacy", 1));
        let state = state_with_repo(repo);
        let service = CategoryService::new(&state);

        let payload = NewCategory {
            id: None,
            title: "Updated".into(),
            description: "New description".into(),
            tag: "NEW".into(),
            project_id: 5,
        };

        let updated = service.update(&actor(), 1, payload).unwrap();
        assert_eq!(updated.title, "Updated");
        assert_eq!(updated.description, "New description");
        assert_eq!(updated.tag, "NEW");
        assert_eq!(updated.project_id, 5);
    }

    #[test]
    fn delete_removes_category() {
        let mut repo = DieselRepoMock::default();
        repo.categories.insert(3, category(3, "Obsolete", 1));
        let state = state_with_repo(repo);
        let service = CategoryService::new(&state);

        let removed = service.delete(&actor(), 3).unwrap();
        assert_eq!(removed.id, 3);
        assert!(matches!(service.get_by_id(3), Err(RepoError::NotFound)));
    }

    #[test]
    fn list_by_project_filters_categories() {
        let mut repo = DieselRepoMock::default();
        repo.categories.insert(1, category(1, "A", 10));
        repo.categories.insert(2, category(2, "B", 20));
        let state = state_with_repo(repo);
        let service = CategoryService::new(&state);

        let cats = service.list_by_project(10).unwrap();
        assert_eq!(cats.len(), 1);
        assert_eq!(cats[0].title, "A");
    }

    #[test]
    fn list_all_returns_all_categories() {
        let mut repo = DieselRepoMock::default();
        repo.categories.insert(1, category(1, "A", 1));
        repo.categories.insert(2, category(2, "B", 2));
        let state = state_with_repo(repo);
        let service = CategoryService::new(&state);

        let cats = service.list_all().unwrap();
        assert_eq!(cats.len(), 2);
    }

    #[test]
    fn list_all_returns_empty_when_no_categories() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = CategoryService::new(&state);

        let cats = service.list_all().unwrap();
        assert_eq!(cats.len(), 0);
    }

    #[test]
    fn list_by_project_returns_empty_for_nonexistent_project() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = CategoryService::new(&state);

        let cats = service.list_by_project(999).unwrap();
        assert_eq!(cats.len(), 0);
    }

    #[test]
    fn get_by_id_returns_not_found_for_nonexistent_category() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = CategoryService::new(&state);

        let result = service.get_by_id(999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn get_category_name_returns_title() {
        let mut repo = DieselRepoMock::default();
        repo.categories.insert(1, category(1, "Functional", 1));
        let state = state_with_repo(repo);
        let service = CategoryService::new(&state);

        let name = service.get_category_name(1).unwrap();
        assert_eq!(name, "Functional");
    }

    #[test]
    fn get_category_name_returns_not_found_for_missing_category() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = CategoryService::new(&state);

        let result = service.get_category_name(999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn update_returns_not_found_for_missing_category() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = CategoryService::new(&state);

        let payload = NewCategory {
            id: None,
            title: "Updated".into(),
            description: "Desc".into(),
            tag: "TAG".into(),
            project_id: 1,
        };

        let result = service.update(&actor(), 999, payload);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn delete_returns_not_found_for_missing_category() {
        let repo = DieselRepoMock::default();
        let state = state_with_repo(repo);
        let service = CategoryService::new(&state);

        let result = service.delete(&actor(), 999);
        assert!(matches!(result, Err(RepoError::NotFound)));
    }

    #[test]
    fn list_by_project_returns_multiple_categories_for_same_project() {
        let mut repo = DieselRepoMock::default();
        repo.categories.insert(1, category(1, "A", 10));
        repo.categories.insert(2, category(2, "B", 10));
        repo.categories.insert(3, category(3, "C", 20));
        let state = state_with_repo(repo);
        let service = CategoryService::new(&state);

        let cats = service.list_by_project(10).unwrap();
        assert_eq!(cats.len(), 2);
        let titles: Vec<&str> = cats.iter().map(|c| c.title.as_str()).collect();
        assert!(titles.contains(&"A"));
        assert!(titles.contains(&"B"));
    }
}
