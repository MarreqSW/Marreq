//! Service for project-scoped custom field definitions.

use crate::app::{AppState, DieselCachedRepo};
use crate::models::{CustomFieldDefinition, CustomFieldDefinitionPayload};
use crate::repository::errors::RepoError;
use crate::repository::CustomFieldRepository;

pub struct CustomFieldService<'a> {
    state: &'a AppState<DieselCachedRepo>,
}

impl<'a> CustomFieldService<'a> {
    pub fn new(state: &'a AppState<DieselCachedRepo>) -> Self {
        Self { state }
    }

    pub fn list_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<CustomFieldDefinition>, RepoError> {
        self.state
            .repo_read()
            .list_custom_field_definitions_by_project(project_id)
    }

    pub fn get_by_id(&self, id: i32) -> Result<CustomFieldDefinition, RepoError> {
        self.state.repo_read().get_custom_field_definition_by_id(id)
    }

    pub fn create(
        &self,
        project_id: i32,
        payload: CustomFieldDefinitionPayload,
    ) -> Result<i32, RepoError> {
        let mut repo = self.state.repo_write();
        repo.create_custom_field_definition(project_id, &payload)
    }

    pub fn update(&self, id: i32, payload: CustomFieldDefinitionPayload) -> Result<(), RepoError> {
        let mut repo = self.state.repo_write();
        repo.update_custom_field_definition(id, &payload)
    }

    pub fn count_versions_using_field(&self, field_id: i32) -> Result<i64, RepoError> {
        self.state
            .repo_read()
            .count_requirement_versions_using_field(field_id)
    }

    pub fn delete(&self, id: i32) -> Result<(), RepoError> {
        let mut repo = self.state.repo_write();
        repo.delete_custom_field_definition(id)
    }
}
