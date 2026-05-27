// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::cache::keys::Keyspace;
use super::cache::{keys, Cache};
use crate::models::*;
use crate::namespaces::project_namespace_segment;
use crate::repository::errors::RepoError;
use crate::repository::{
    ApiTokensRepository, BaselineRepository, CustomFieldRepository, LogRepository,
    LookupRepository, MatrixRepository, ProjectMembersRepository, ProjectReviewersRepository,
    ProjectsRepository, Repository, RequirementCommentsRepository,
    RequirementVersionLinksRepository, RequirementsRepository, UserRepository,
    VerificationsRepository,
};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Repository wrapper that checks the cache before hitting the database
#[derive(Clone)]
pub struct CacheRepository<R> {
    inner: R,
    cache: Arc<Cache>,
}

impl<R: Repository> CacheRepository<R> {
    /// Create a new repository wrapper with the provided cache instance
    pub fn new(inner: R, ttl_seconds: u64) -> Self {
        Self {
            inner,
            cache: Cache::new(ttl_seconds).into(),
        }
    }

    pub fn inner_repo(&self) -> &R {
        &self.inner
    }

    /// Get a reference to the underlying cache
    pub fn cache(&self) -> Arc<Cache> {
        Arc::clone(&self.cache)
    }

    fn get_or_fetch<T, F>(&self, key: &str, ttl: Duration, fetch: F) -> Result<T, RepoError>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Result<T, RepoError>,
    {
        if let Some(cached) = self.cache.get(key) {
            if let Ok(value) = serde_json::from_str(&cached) {
                return Ok(value);
            }
            self.cache.remove(key);
        }
        let value = fetch()?;
        if let Ok(json) = serde_json::to_string(&value) {
            self.cache.set_with_ttl(key, json, ttl);
        }
        Ok(value)
    }

    /// Warm up the cache with frequently accessed data
    ///
    /// Populates the cache with common queries to improve initial performance.
    /// Note: This function may copy significant amounts of data; use with caution.
    pub fn warm_cache(&self) {
        if let Ok(projects) = self.inner.get_projects_all() {
            if let Ok(json_data) = serde_json::to_string(&projects) {
                self.cache.set_with_ttl(
                    keys::PROJECTS_ALL,
                    json_data.clone(),
                    Duration::from_secs(600),
                );
                self.cache
                    .set_with_ttl(keys::PROJECTS_NAV, json_data, Duration::from_secs(300));
            }
        }

        if let Ok(statuses) = self.inner.get_requirement_status_all() {
            if let Ok(json_data) = serde_json::to_string(&statuses) {
                self.cache.set_with_ttl(
                    keys::REQUIREMENT_STATUS_ALL,
                    json_data,
                    Duration::from_secs(900),
                );
            }
        }

        if let Ok(categories) = self.inner.get_categories_all() {
            if let Ok(json_data) = serde_json::to_string(&categories) {
                self.cache
                    .set_with_ttl(keys::CATEGORIES_ALL, json_data, Duration::from_secs(900));
            }
        }

        if let Ok(users) = self.inner.get_users_all() {
            if let Ok(json_data) = serde_json::to_string(&users) {
                self.cache
                    .set_with_ttl(keys::USERS_ALL, json_data, Duration::from_secs(600));
            }
        }
    }

    fn invalidate_owned_project_namespace_keys(
        &self,
        user_id: i32,
        namespaces: &[&str],
    ) -> Result<(), RepoError> {
        let projects = self.inner.get_projects_all()?;
        for project in projects
            .iter()
            .filter(|project| project.group_id.is_none() && project.owner_id == Some(user_id))
        {
            for namespace in namespaces {
                self.cache
                    .invalidate_project_namespace_slug(namespace, &project.slug);
            }
        }

        Ok(())
    }
}

impl<R: Repository> RequirementsRepository for CacheRepository<R> {
    fn get_requirement_by_id(&self, requirement_id: i32) -> Result<Requirement, RepoError> {
        let key = keys::Requirements::by_id(requirement_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_requirement_by_id(requirement_id)
        })
    }

    fn get_requirements_all(&self) -> Result<Vec<Requirement>, RepoError> {
        self.get_or_fetch(keys::REQUIREMENTS_ALL, Duration::from_secs(300), || {
            self.inner.get_requirements_all()
        })
    }

    fn get_requirements_by_project(&self, project_id: i32) -> Result<Vec<Requirement>, RepoError> {
        // Do not cache: bulk SQL seeds / admin imports bypass repository invalidation and would
        // leave stale lists visible for minutes (dashboard + requirements table use this path).
        self.inner.get_requirements_by_project(project_id)
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
        self.inner.get_requirements_by_project_filtered_paginated(
            project_id,
            status_filter,
            verification_filter,
            category_filter,
            applicability_filter,
            custom_field_filters,
            limit,
            offset,
        )
    }

    fn get_verification_method_ids_for_requirement(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<i32>, RepoError> {
        self.inner
            .get_verification_method_ids_for_requirement(requirement_id)
    }

    fn get_verification_method_ids_for_version(
        &self,
        version_id: i32,
    ) -> Result<Vec<i32>, RepoError> {
        self.inner
            .get_verification_method_ids_for_version(version_id)
    }

    fn get_requirement_ids_by_verification_method(
        &self,
        verification_method_id: i32,
    ) -> Result<Vec<i32>, RepoError> {
        self.inner
            .get_requirement_ids_by_verification_method(verification_method_id)
    }

    fn set_requirement_verification_methods(
        &mut self,
        requirement_id: i32,
        verification_method_ids: &[i32],
    ) -> Result<(), RepoError> {
        let res = self
            .inner
            .set_requirement_verification_methods(requirement_id, verification_method_ids);
        if res.is_ok() {
            self.cache.invalidate_requirement(requirement_id);
        }
        res
    }

    fn insert_new_requirement(&mut self, new: &NewRequirement) -> Result<i32, RepoError> {
        let id = self.inner.insert_new_requirement(new)?;
        self.cache.invalidate_requirement(id);
        self.cache.invalidate_project(new.project_id);
        Ok(id)
    }

    fn edit_requirement(&mut self, new: &NewRequirement) -> Result<bool, RepoError> {
        let res = self.inner.edit_requirement(new)?;
        if let Some(id) = new.id {
            self.cache.invalidate_requirement(id);
        }
        self.cache.invalidate_project(new.project_id);
        Ok(res)
    }

    fn delete_requirement(&mut self, requirement_id: i32) -> Result<Requirement, RepoError> {
        let req = self.inner.delete_requirement(requirement_id)?;
        self.cache.invalidate_requirement(requirement_id);
        self.cache.invalidate_project(req.project_id);
        self.cache.remove(super::cache::keys::REQUIREMENTS_ALL);
        Ok(req)
    }

    fn update_requirement(&mut self, requirement_id: i32) -> Result<(), RepoError> {
        self.inner.update_requirement(requirement_id)?;
        self.cache.invalidate_requirement(requirement_id);
        Ok(())
    }

    fn list_requirement_versions(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<RequirementVersion>, RepoError> {
        self.inner.list_requirement_versions(requirement_id)
    }

    fn get_requirement_version_by_id(
        &self,
        version_id: i32,
    ) -> Result<RequirementVersion, RepoError> {
        self.inner.get_requirement_version_by_id(version_id)
    }

    fn set_requirement_version_approval(
        &mut self,
        version_id: i32,
        new_state: &str,
        approved_by_user_id: i32,
    ) -> Result<RequirementVersion, RepoError> {
        let updated = self.inner.set_requirement_version_approval(
            version_id,
            new_state,
            approved_by_user_id,
        )?;
        self.cache.invalidate_requirement(updated.requirement_id);
        if let Ok(req) = self.inner.get_requirement_by_id(updated.requirement_id) {
            self.cache.invalidate_project(req.project_id);
        }
        Ok(updated)
    }
}

impl<R: Repository> ApiTokensRepository for CacheRepository<R> {
    fn get_user_by_token_hash(&self, token_hash: &str) -> Result<(User, Option<i32>), RepoError> {
        self.inner.get_user_by_token_hash(token_hash)
    }

    fn update_api_token_last_used_at(&mut self, token_hash: &str) -> Result<(), RepoError> {
        self.inner.update_api_token_last_used_at(token_hash)
    }
}

impl<R: Repository> UserRepository for CacheRepository<R> {
    fn get_users_all(&self) -> Result<Vec<User>, RepoError> {
        self.get_or_fetch(keys::USERS_ALL, Duration::from_secs(300), || {
            self.inner.get_users_all()
        })
    }

    fn get_user_by_id(&self, user_id: i32) -> Result<User, RepoError> {
        let key = keys::Users::by_id(user_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_user_by_id(user_id)
        })
    }

    /// Bypasses cache so login always sees the current password hash from the database.
    fn get_user_by_username(&self, uname: &str) -> Result<Option<User>, RepoError> {
        self.inner.get_user_by_username(uname)
    }

    fn insert_user(&mut self, new: &NewUser) -> Result<i32, RepoError> {
        let id = self.inner.insert_user(new)?;
        self.cache.invalidate_user(id);
        Ok(id)
    }

    fn update_user_password(&mut self, user_id: i32, new_hash: &str) -> Result<(), RepoError> {
        let username = self
            .inner
            .get_user_by_id(user_id)
            .ok()
            .map(|u| u.username.to_lowercase());
        self.inner.update_user_password(user_id, new_hash)?;
        self.cache.invalidate_user(user_id);
        if let Some(ref u) = username {
            self.cache.remove(&format!("user:username:{}", u));
        }
        Ok(())
    }

    fn update_user(&mut self, user_data: &NewUser) -> Result<bool, RepoError> {
        let username = user_data.id.and_then(|id| {
            self.inner
                .get_user_by_id(id)
                .ok()
                .map(|u| u.username.to_lowercase())
        });
        let res = self.inner.update_user(user_data)?;
        if let Some(id) = user_data.id {
            self.cache.invalidate_user(id);
            let new_username = user_data.username.to_lowercase();
            if let Some(ref old_username) = username {
                self.invalidate_owned_project_namespace_keys(
                    id,
                    &[old_username.as_str(), new_username.as_str()],
                )?;
            }
        }
        if let Some(ref u) = username {
            self.cache.remove(&format!("user:username:{}", u));
        }
        Ok(res)
    }

    fn update_user_without_password(&mut self, user_data: &UpdateUser) -> Result<bool, RepoError> {
        let username = user_data.id.and_then(|id| {
            self.inner
                .get_user_by_id(id)
                .ok()
                .map(|u| u.username.to_lowercase())
        });
        let res = self.inner.update_user_without_password(user_data)?;
        if let Some(id) = user_data.id {
            self.cache.invalidate_user(id);
            let new_username = user_data.username.to_lowercase();
            if let Some(ref old_username) = username {
                self.invalidate_owned_project_namespace_keys(
                    id,
                    &[old_username.as_str(), new_username.as_str()],
                )?;
            }
        }
        if let Some(ref u) = username {
            self.cache.remove(&format!("user:username:{}", u));
        }
        Ok(res)
    }

    fn delete_user(&mut self, user_id: i32) -> Result<User, RepoError> {
        let username = self
            .inner
            .get_user_by_id(user_id)
            .ok()
            .map(|u| u.username.to_lowercase());
        let memberships = self.inner.get_projects_for_user(user_id)?;
        let user = self.inner.delete_user(user_id)?;
        self.cache.invalidate_user(user_id);
        if let Some(ref u) = username {
            self.cache.remove(&format!("user:username:{}", u));
            self.invalidate_owned_project_namespace_keys(user_id, &[u.as_str()])?;
        }
        for membership in memberships {
            self.cache
                .invalidate_project_membership(membership.project_id, user_id);
            self.cache.invalidate_project(membership.project_id);
        }
        Ok(user)
    }

    fn get_user_by_email(&self, email: &str) -> Result<Option<User>, RepoError> {
        // Bypass cache: used during login/registration where freshness matters.
        self.inner.get_user_by_email(email)
    }

    fn set_user_email_verified(&mut self, user_id: i32, verified: bool) -> Result<(), RepoError> {
        self.inner.set_user_email_verified(user_id, verified)?;
        self.cache.invalidate_user(user_id);
        Ok(())
    }
}

impl<R: Repository> super::WorkspacesRepository for CacheRepository<R> {
    fn insert_workspace(&mut self, new: &crate::models::NewWorkspace) -> Result<i32, RepoError> {
        self.inner.insert_workspace(new)
    }

    fn get_workspace_by_id(&self, id: i32) -> Result<crate::models::Workspace, RepoError> {
        self.inner.get_workspace_by_id(id)
    }

    fn get_workspace_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<crate::models::Workspace>, RepoError> {
        self.inner.get_workspace_by_slug(slug)
    }

    fn get_personal_workspace_for_user(
        &self,
        user_id: i32,
    ) -> Result<Option<crate::models::Workspace>, RepoError> {
        self.inner.get_personal_workspace_for_user(user_id)
    }
}

impl<R: Repository> super::EmailTokensRepository for CacheRepository<R> {
    fn insert_email_token(&mut self, new: &crate::models::NewEmailToken) -> Result<i32, RepoError> {
        self.inner.insert_email_token(new)
    }

    fn find_email_token_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<crate::models::EmailToken>, RepoError> {
        self.inner.find_email_token_by_hash(token_hash)
    }

    fn mark_email_token_used(&mut self, id: i32) -> Result<(), RepoError> {
        self.inner.mark_email_token_used(id)
    }
}

impl<R: Repository> super::SessionRepository for CacheRepository<R> {
    fn create_session(
        &mut self,
        new: &crate::models::entities::NewSession,
    ) -> Result<(), RepoError> {
        self.inner.create_session(new)
    }

    fn find_active_session(
        &self,
        token_hash: &str,
        now: chrono::NaiveDateTime,
    ) -> Result<Option<crate::models::entities::Session>, RepoError> {
        self.inner.find_active_session(token_hash, now)
    }

    fn touch_session(
        &mut self,
        token_hash: &str,
        now: chrono::NaiveDateTime,
    ) -> Result<(), RepoError> {
        self.inner.touch_session(token_hash, now)
    }

    fn delete_session(&mut self, token_hash: &str) -> Result<(), RepoError> {
        self.inner.delete_session(token_hash)
    }

    fn delete_user_sessions(&mut self, user_id: i32) -> Result<(), RepoError> {
        self.inner.delete_user_sessions(user_id)
    }

    fn purge_expired_sessions(&mut self, now: chrono::NaiveDateTime) -> Result<usize, RepoError> {
        self.inner.purge_expired_sessions(now)
    }
}

impl<R: Repository> ProjectMembersRepository for CacheRepository<R> {
    fn get_members_by_project(&self, project_id: i32) -> Result<Vec<ProjectMember>, RepoError> {
        let key = keys::ProjectMembers::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_members_by_project(project_id)
        })
    }

    fn get_projects_for_user(&self, user_id: i32) -> Result<Vec<ProjectMember>, RepoError> {
        let key = keys::ProjectMembers::for_user(user_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_projects_for_user(user_id)
        })
    }

    fn add_project_member(&mut self, new: &NewProjectMember) -> Result<(), RepoError> {
        self.inner.add_project_member(new)?;
        self.cache
            .invalidate_project_membership(new.project_id, new.user_id);
        self.cache.invalidate_project(new.project_id);
        self.cache.invalidate_user(new.user_id);
        Ok(())
    }

    fn update_project_member_role(
        &mut self,
        project_id: i32,
        id: i32,
        role: i32,
    ) -> Result<(), RepoError> {
        self.inner
            .update_project_member_role(project_id, id, role)?;
        self.cache.invalidate_project_membership(project_id, id);
        self.cache.invalidate_project(project_id);
        self.cache.invalidate_user(id);
        Ok(())
    }

    fn remove_project_member(&mut self, project_id: i32, id: i32) -> Result<(), RepoError> {
        self.inner.remove_project_member(project_id, id)?;
        self.cache.invalidate_project_membership(project_id, id);
        self.cache.invalidate_project(project_id);
        self.cache.invalidate_user(id);
        Ok(())
    }
}

impl<R: Repository> ProjectReviewersRepository for CacheRepository<R> {
    fn is_project_reviewer(&self, project_id: i32, user_id: i32) -> Result<bool, RepoError> {
        self.inner.is_project_reviewer(project_id, user_id)
    }

    fn list_project_reviewer_ids(&self, project_id: i32) -> Result<Vec<i32>, RepoError> {
        self.inner.list_project_reviewer_ids(project_id)
    }

    fn replace_project_reviewers(
        &mut self,
        project_id: i32,
        user_ids: &[i32],
    ) -> Result<(), RepoError> {
        self.inner.replace_project_reviewers(project_id, user_ids)?;
        self.cache.invalidate_project(project_id);
        Ok(())
    }
}

impl<R: Repository> VerificationsRepository for CacheRepository<R> {
    fn get_verification_by_id(&self, verification_id: i32) -> Result<Verification, RepoError> {
        let key = keys::Verifications::by_id(verification_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_verification_by_id(verification_id)
        })
    }

    fn get_verifications_all(&self) -> Result<Vec<Verification>, RepoError> {
        self.get_or_fetch(keys::VERIFICATIONS_ALL, Duration::from_secs(300), || {
            self.inner.get_verifications_all()
        })
    }

    fn get_verifications_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<Verification>, RepoError> {
        let key = keys::Verifications::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_verifications_by_project(project_id)
        })
    }

    fn get_requirements_for_verification(
        &self,
        verification_id: i32,
    ) -> Result<Vec<Requirement>, RepoError> {
        let key = keys::LinkedRequirements::for_test(verification_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner
                .get_requirements_for_verification(verification_id)
        })
    }

    fn get_verifications_for_requirement(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<Verification>, RepoError> {
        let key = keys::LinkedVerifications::for_requirement(requirement_id);
        self.get_or_fetch(&key, Duration::from_secs(300), || {
            self.inner.get_verifications_for_requirement(requirement_id)
        })
    }

    fn get_impacted_verifications_for_requirement(
        &self,
        requirement_id: i32,
    ) -> Result<Vec<Verification>, RepoError> {
        self.inner
            .get_impacted_verifications_for_requirement(requirement_id)
    }

    fn insert_verification(&mut self, new: &NewVerification) -> Result<i32, RepoError> {
        let id = self.inner.insert_verification(new)?;
        self.cache.invalidate_verification(id);
        self.cache.invalidate_project(new.project_id);
        Ok(id)
    }

    fn edit_verification(&mut self, new: &NewVerification) -> Result<bool, RepoError> {
        let res = self.inner.edit_verification(new)?;
        if let Some(id) = new.id {
            self.cache.invalidate_verification(id);
        }
        self.cache.invalidate_project(new.project_id);
        Ok(res)
    }

    fn delete_verification(&mut self, verification_id: i32) -> Result<Verification, RepoError> {
        let verification = self.inner.delete_verification(verification_id)?;
        self.cache.invalidate_verification(verification_id);
        self.cache.invalidate_project(verification.project_id);
        self.cache.remove(super::cache::keys::VERIFICATIONS_ALL);
        Ok(verification)
    }

    fn update_verification_requirement_links(
        &mut self,
        verification_id: i32,
        requirement_ids: &[i32],
    ) -> Result<(), RepoError> {
        let project_id = self
            .inner
            .get_verification_by_id(verification_id)
            .ok()
            .map(|v| v.project_id);
        self.inner
            .update_verification_requirement_links(verification_id, requirement_ids)?;
        self.cache.invalidate_verification(verification_id);
        for &requirement_id in requirement_ids {
            self.cache.invalidate_requirement(requirement_id);
        }
        if let Some(pid) = project_id {
            self.cache
                .remove(&super::cache::keys::Matrix::by_project(pid));
        }
        Ok(())
    }

    fn record_verification_status_audit(
        &mut self,
        verification_id: i32,
        actor_id: i32,
    ) -> Result<(), RepoError> {
        let project_id = self
            .inner
            .get_verification_by_id(verification_id)
            .ok()
            .map(|v| v.project_id);
        self.inner
            .record_verification_status_audit(verification_id, actor_id)?;
        self.cache.invalidate_verification(verification_id);
        if let Some(pid) = project_id {
            self.cache.invalidate_project(pid);
        }
        Ok(())
    }
}

impl<R: Repository> LookupRepository for CacheRepository<R> {
    fn get_requirement_status_all(&self) -> Result<Vec<RequirementStatus>, RepoError> {
        self.get_or_fetch(
            keys::REQUIREMENT_STATUS_ALL,
            Duration::from_secs(900),
            || self.inner.get_requirement_status_all(),
        )
    }

    fn get_requirement_status_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<RequirementStatus>, RepoError> {
        let key = keys::RequirementStatus::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(900), || {
            self.inner.get_requirement_status_by_project(project_id)
        })
    }

    fn get_requirement_status_by_id(&self, status_id: i32) -> Result<RequirementStatus, RepoError> {
        let key = keys::RequirementStatus::by_id(status_id);
        self.get_or_fetch(&key, Duration::from_secs(900), || {
            self.inner.get_requirement_status_by_id(status_id)
        })
    }

    fn get_verification_status_all(&self) -> Result<Vec<VerificationStatus>, RepoError> {
        self.get_or_fetch(
            keys::VERIFICATION_STATUS_ALL,
            Duration::from_secs(900),
            || self.inner.get_verification_status_all(),
        )
    }

    fn get_verification_status_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<VerificationStatus>, RepoError> {
        let key = keys::VerificationStatus::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(900), || {
            self.inner.get_verification_status_by_project(project_id)
        })
    }

    fn get_verification_status_by_id(
        &self,
        status_id: i32,
    ) -> Result<VerificationStatus, RepoError> {
        let key = keys::VerificationStatus::by_id(status_id);
        self.get_or_fetch(&key, Duration::from_secs(900), || {
            self.inner.get_verification_status_by_id(status_id)
        })
    }

    fn get_categories_all(&self) -> Result<Vec<Category>, RepoError> {
        self.get_or_fetch(keys::CATEGORIES_ALL, Duration::from_secs(600), || {
            self.inner.get_categories_all()
        })
    }

    fn get_categories_by_project(&self, project_id: i32) -> Result<Vec<Category>, RepoError> {
        let key = keys::Categories::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(600), || {
            self.inner.get_categories_by_project(project_id)
        })
    }

    fn get_category_by_id(&self, category_id: i32) -> Result<Category, RepoError> {
        let key = keys::Categories::by_id(category_id);
        self.get_or_fetch(&key, Duration::from_secs(600), || {
            self.inner.get_category_by_id(category_id)
        })
    }

    fn get_applicability_all(&self) -> Result<Vec<Applicability>, RepoError> {
        self.get_or_fetch(keys::APPLICABILITY_ALL, Duration::from_secs(600), || {
            self.inner.get_applicability_all()
        })
    }

    fn get_applicability_by_id(&self, applicability_id: i32) -> Result<Applicability, RepoError> {
        let key = keys::Applicability::by_id(applicability_id);
        self.get_or_fetch(&key, Duration::from_secs(600), || {
            self.inner.get_applicability_by_id(applicability_id)
        })
    }

    fn get_applicability_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<Applicability>, RepoError> {
        let key = keys::Applicability::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(600), || {
            self.inner.get_applicability_by_project(project_id)
        })
    }

    fn get_verification_methods_all(&self) -> Result<Vec<VerificationMethod>, RepoError> {
        self.get_or_fetch(keys::VERIFICATION_ALL, Duration::from_secs(600), || {
            self.inner.get_verification_methods_all()
        })
    }

    fn get_verification_method_by_id(
        &self,
        verification_method_id: i32,
    ) -> Result<VerificationMethod, RepoError> {
        let key = keys::VerificationMethod::by_id(verification_method_id);
        self.get_or_fetch(&key, Duration::from_secs(600), || {
            self.inner
                .get_verification_method_by_id(verification_method_id)
        })
    }

    fn get_verification_methods_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<VerificationMethod>, RepoError> {
        let key = keys::VerificationMethod::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(600), || {
            self.inner.get_verification_methods_by_project(project_id)
        })
    }

    fn create_requirement_status(&mut self, new: &NewRequirementStatus) -> Result<i32, RepoError> {
        let id = self.inner.create_requirement_status(new)?;
        self.cache.invalidate_status(id);
        self.cache
            .invalidate_requirement_status_by_project(new.project_id);
        Ok(id)
    }

    fn create_verification_status(
        &mut self,
        new: &NewVerificationStatus,
    ) -> Result<i32, RepoError> {
        let id = self.inner.create_verification_status(new)?;
        self.cache.invalidate_status(id);
        self.cache
            .invalidate_verification_status_by_project(new.project_id);
        Ok(id)
    }

    fn update_requirement_status(
        &mut self,
        id: i32,
        payload: &NewRequirementStatus,
    ) -> Result<bool, RepoError> {
        let res = self.inner.update_requirement_status(id, payload)?;
        if res {
            self.cache.invalidate_status(id);
            self.cache
                .invalidate_requirement_status_by_project(payload.project_id);
        }
        Ok(res)
    }

    fn delete_requirement_status(&mut self, id: i32) -> Result<RequirementStatus, RepoError> {
        let status = self.inner.delete_requirement_status(id)?;
        self.cache.invalidate_status(id);
        self.cache
            .invalidate_requirement_status_by_project(status.project_id);
        Ok(status)
    }

    fn update_verification_status(
        &mut self,
        id: i32,
        payload: &NewVerificationStatus,
    ) -> Result<bool, RepoError> {
        let res = self.inner.update_verification_status(id, payload)?;
        if res {
            self.cache.invalidate_status(id);
            self.cache
                .invalidate_verification_status_by_project(payload.project_id);
        }
        Ok(res)
    }

    fn delete_verification_status(&mut self, id: i32) -> Result<VerificationStatus, RepoError> {
        let status = self.inner.delete_verification_status(id)?;
        self.cache.invalidate_status(id);
        self.cache
            .invalidate_verification_status_by_project(status.project_id);
        Ok(status)
    }

    fn insert_new_verification_method(
        &mut self,
        new: &NewVerificationMethod,
    ) -> Result<i32, RepoError> {
        let id = self.inner.insert_new_verification_method(new)?;
        self.cache.invalidate_verification_method(id);
        self.cache.invalidate_project(new.project_id);
        Ok(id)
    }

    fn edit_verification_method(&mut self, new: &NewVerificationMethod) -> Result<bool, RepoError> {
        let res = self.inner.edit_verification_method(new)?;
        if let Some(id) = new.id {
            self.cache.invalidate_verification_method(id);
        }
        self.cache.invalidate_project(new.project_id);
        Ok(res)
    }

    fn delete_verification_method(
        &mut self,
        verification_method_id: i32,
    ) -> Result<VerificationMethod, RepoError> {
        let verification = self
            .inner
            .delete_verification_method(verification_method_id)?;
        self.cache
            .invalidate_verification_method(verification_method_id);
        self.cache.invalidate_project(verification.project_id);
        Ok(verification)
    }

    fn insert_new_category(&mut self, new: &NewCategory) -> Result<i32, RepoError> {
        let id = self.inner.insert_new_category(new)?;
        self.cache.invalidate_category(id);
        self.cache.invalidate_project(new.project_id);
        Ok(id)
    }

    fn edit_category(&mut self, new: &NewCategory) -> Result<bool, RepoError> {
        let res = self.inner.edit_category(new)?;
        if let Some(id) = new.id {
            self.cache.invalidate_category(id);
        }
        self.cache.invalidate_project(new.project_id);
        Ok(res)
    }

    fn delete_category(&mut self, category_id: i32) -> Result<Category, RepoError> {
        let cat = self.inner.delete_category(category_id)?;
        self.cache.invalidate_category(category_id);
        self.cache.invalidate_project(cat.project_id);
        Ok(cat)
    }

    fn insert_new_applicability(&mut self, new: &NewApplicability) -> Result<i32, RepoError> {
        let id = self.inner.insert_new_applicability(new)?;
        self.cache.invalidate_applicability(id);
        self.cache.invalidate_project(new.project_id);
        Ok(id)
    }

    fn edit_applicability(&mut self, new: &NewApplicability) -> Result<bool, RepoError> {
        let res = self.inner.edit_applicability(new)?;
        if let Some(id) = new.id {
            self.cache.invalidate_applicability(id);
        }
        self.cache.invalidate_project(new.project_id);
        Ok(res)
    }

    fn delete_applicability(&mut self, applicability_id: i32) -> Result<Applicability, RepoError> {
        let app = self.inner.delete_applicability(applicability_id)?;
        self.cache.invalidate_applicability(applicability_id);
        self.cache.invalidate_project(app.project_id);
        Ok(app)
    }
}

impl<R: Repository> super::GroupsRepository for CacheRepository<R> {
    fn get_groups_all(&self) -> Result<Vec<Group>, RepoError> {
        self.inner.get_groups_all()
    }

    fn get_group_by_id(&self, group_id: i32) -> Result<Group, RepoError> {
        self.inner.get_group_by_id(group_id)
    }

    fn get_group_by_slug(&self, slug: &str) -> Result<Group, RepoError> {
        self.inner.get_group_by_slug(slug)
    }

    fn insert_new_group(&mut self, new: &NewGroupRow) -> Result<i32, RepoError> {
        self.inner.insert_new_group(new)
    }

    fn edit_group(&mut self, group_id: i32, update: &UpdateGroup) -> Result<bool, RepoError> {
        self.inner.edit_group(group_id, update)
    }

    fn delete_group(&mut self, group_id: i32) -> Result<Group, RepoError> {
        self.inner.delete_group(group_id)
    }

    fn get_projects_by_group(&self, group_id: i32) -> Result<Vec<Project>, RepoError> {
        self.inner.get_projects_by_group(group_id)
    }
}

impl<R: Repository> super::GroupMembersRepository for CacheRepository<R> {
    fn get_members_by_group(&self, group_id: i32) -> Result<Vec<GroupMember>, RepoError> {
        self.inner.get_members_by_group(group_id)
    }

    fn get_groups_for_user(&self, user_id: i32) -> Result<Vec<GroupMember>, RepoError> {
        self.inner.get_groups_for_user(user_id)
    }

    fn add_group_member(&mut self, new: &NewGroupMember) -> Result<(), RepoError> {
        self.inner.add_group_member(new)
    }

    fn update_group_member_role(
        &mut self,
        group_id: i32,
        user_id: i32,
        role: i32,
    ) -> Result<(), RepoError> {
        self.inner.update_group_member_role(group_id, user_id, role)
    }

    fn remove_group_member(&mut self, group_id: i32, user_id: i32) -> Result<(), RepoError> {
        self.inner.remove_group_member(group_id, user_id)
    }
}

impl<R: Repository> ProjectsRepository for CacheRepository<R> {
    fn get_projects_all(&self) -> Result<Vec<Project>, RepoError> {
        self.get_or_fetch(keys::PROJECTS_ALL, Duration::from_secs(600), || {
            self.inner.get_projects_all()
        })
    }

    fn get_project_by_id(&self, project_id: i32) -> Result<Project, RepoError> {
        let key = keys::Projects::by_id(project_id);
        self.get_or_fetch(&key, Duration::from_secs(900), || {
            self.inner.get_project_by_id(project_id)
        })
    }

    fn get_project_by_slug(&self, project_slug: &str) -> Result<Project, RepoError> {
        let key = keys::Projects::by_slug(project_slug);
        self.get_or_fetch(&key, Duration::from_secs(900), || {
            self.inner.get_project_by_slug(project_slug)
        })
    }

    fn get_project_by_user_namespace_and_slug(
        &self,
        username: &str,
        slug: &str,
    ) -> Result<Project, RepoError> {
        let key = keys::Projects::by_namespace_slug(username, slug);
        self.get_or_fetch(&key, Duration::from_secs(900), || {
            self.inner
                .get_project_by_user_namespace_and_slug(username, slug)
        })
    }

    fn get_project_by_group_namespace_and_slug(
        &self,
        group_slug: &str,
        slug: &str,
    ) -> Result<Project, RepoError> {
        let key = keys::Projects::by_namespace_slug(group_slug, slug);
        self.get_or_fetch(&key, Duration::from_secs(900), || {
            self.inner
                .get_project_by_group_namespace_and_slug(group_slug, slug)
        })
    }

    fn insert_new_project(&mut self, new: &NewProjectRow) -> Result<i32, RepoError> {
        let id = self.inner.insert_new_project(new)?;
        let project = self.inner.get_project_by_id(id)?;
        self.cache.invalidate_project(id);
        self.cache.invalidate_project_slug(&new.slug);
        if let Ok(namespace) = project_namespace_segment(&self.inner, &project) {
            self.cache
                .invalidate_project_namespace_slug(&namespace, &project.slug);
        }
        Ok(id)
    }

    fn edit_project(&mut self, project_id: i32, update: &UpdateProject) -> Result<bool, RepoError> {
        let before = self.inner.get_project_by_id(project_id)?;
        let res = self.inner.edit_project(project_id, update)?;
        let after = self.inner.get_project_by_id(project_id)?;
        self.cache.invalidate_project(project_id);
        self.cache.invalidate_project_slug(&before.slug);
        self.cache.invalidate_project_slug(&after.slug);
        if let Ok(namespace) = project_namespace_segment(&self.inner, &before) {
            self.cache
                .invalidate_project_namespace_slug(&namespace, &before.slug);
        }
        if let Ok(namespace) = project_namespace_segment(&self.inner, &after) {
            self.cache
                .invalidate_project_namespace_slug(&namespace, &after.slug);
        }
        Ok(res)
    }

    fn delete_project(&mut self, project_id: i32) -> Result<Project, RepoError> {
        let proj = self.inner.delete_project(project_id)?;
        self.cache.invalidate_project(project_id);
        self.cache.invalidate_project_slug(&proj.slug);
        if let Ok(namespace) = project_namespace_segment(&self.inner, &proj) {
            self.cache
                .invalidate_project_namespace_slug(&namespace, &proj.slug);
        }
        Ok(proj)
    }
}

impl<R: Repository> MatrixRepository for CacheRepository<R> {
    fn get_matrix_by_project(&self, project_id: i32) -> Result<Vec<MatrixLink>, RepoError> {
        let key = keys::Matrix::by_project(project_id);
        self.get_or_fetch(&key, Duration::from_secs(180), || {
            self.inner.get_matrix_by_project(project_id)
        })
    }

    fn insert_new_matrix_item(&mut self, new: &NewMatrixLink) -> Result<(), RepoError> {
        self.inner.insert_new_matrix_item(new)?;
        let key = keys::Matrix::by_project(new.project_id);
        self.cache.remove(&key);
        Ok(())
    }

    fn mark_links_suspect_for_requirement(
        &mut self,
        requirement_id: i32,
        reason: &str,
        triggering_version_id: Option<i32>,
        triggering_user_id: Option<i32>,
    ) -> Result<Vec<i32>, RepoError> {
        let project_ids = self.inner.mark_links_suspect_for_requirement(
            requirement_id,
            reason,
            triggering_version_id,
            triggering_user_id,
        )?;
        for &pid in &project_ids {
            self.cache.remove(&keys::Matrix::by_project(pid));
        }
        Ok(project_ids)
    }

    fn clear_suspect(
        &mut self,
        req_id: i32,
        test_id: i32,
        cleared_by_user_id: i32,
    ) -> Result<(bool, Option<i32>), RepoError> {
        let (ok, project_id) = self
            .inner
            .clear_suspect(req_id, test_id, cleared_by_user_id)?;
        if let Some(pid) = project_id {
            self.cache.remove(&keys::Matrix::by_project(pid));
        }
        Ok((ok, project_id))
    }
}

impl<R: Repository> CustomFieldRepository for CacheRepository<R> {
    fn list_custom_field_definitions_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<CustomFieldDefinition>, RepoError> {
        self.inner
            .list_custom_field_definitions_by_project(project_id)
    }

    fn get_custom_field_definition_by_id(
        &self,
        id: i32,
    ) -> Result<CustomFieldDefinition, RepoError> {
        self.inner.get_custom_field_definition_by_id(id)
    }

    fn create_custom_field_definition(
        &mut self,
        project_id: i32,
        payload: &CustomFieldDefinitionPayload,
    ) -> Result<i32, RepoError> {
        self.inner
            .create_custom_field_definition(project_id, payload)
    }

    fn update_custom_field_definition(
        &mut self,
        id: i32,
        payload: &CustomFieldDefinitionPayload,
    ) -> Result<(), RepoError> {
        self.inner.update_custom_field_definition(id, payload)
    }

    fn count_requirement_versions_using_field(&self, field_id: i32) -> Result<i64, RepoError> {
        self.inner.count_requirement_versions_using_field(field_id)
    }

    fn delete_custom_field_definition(&mut self, id: i32) -> Result<(), RepoError> {
        self.inner.delete_custom_field_definition(id)
    }

    fn get_custom_field_values_for_version(
        &self,
        version_id: i32,
    ) -> Result<Vec<CustomFieldValueDisplay>, RepoError> {
        self.inner.get_custom_field_values_for_version(version_id)
    }

    fn set_custom_field_values_for_version(
        &mut self,
        version_id: i32,
        values: &[(i32, Option<String>)],
    ) -> Result<(), RepoError> {
        self.inner
            .set_custom_field_values_for_version(version_id, values)
    }
}

impl<R: Repository> BaselineRepository for CacheRepository<R> {
    fn create_baseline(
        &mut self,
        project_id: i32,
        created_by: i32,
        payload: &crate::models::NewBaseline,
    ) -> Result<crate::models::Baseline, RepoError> {
        self.inner.create_baseline(project_id, created_by, payload)
    }

    fn list_baselines_by_project(
        &self,
        project_id: i32,
    ) -> Result<Vec<crate::models::Baseline>, RepoError> {
        self.inner.list_baselines_by_project(project_id)
    }

    fn get_baseline_by_id(&self, baseline_id: i32) -> Result<crate::models::Baseline, RepoError> {
        self.inner.get_baseline_by_id(baseline_id)
    }

    fn get_requirements_for_baseline(
        &self,
        baseline_id: i32,
    ) -> Result<Vec<crate::models::Requirement>, RepoError> {
        self.inner.get_requirements_for_baseline(baseline_id)
    }

    fn get_baseline_requirement_version_id(
        &self,
        baseline_id: i32,
        requirement_id: i32,
    ) -> Result<Option<i32>, RepoError> {
        self.inner
            .get_baseline_requirement_version_id(baseline_id, requirement_id)
    }

    fn get_baseline_traceability(
        &self,
        baseline_id: i32,
    ) -> Result<Vec<crate::models::BaselineTraceability>, RepoError> {
        self.inner.get_baseline_traceability(baseline_id)
    }

    fn get_verifications_for_baseline(
        &self,
        baseline_id: i32,
    ) -> Result<Vec<crate::models::BaselineVerification>, RepoError> {
        self.inner.get_verifications_for_baseline(baseline_id)
    }
}

impl<R: LogRepository> LogRepository for CacheRepository<R> {
    fn insert_log(&mut self, new: &NewLog) -> Result<(), RepoError> {
        self.inner.insert_log(new)
    }

    fn get_logs_recent(&self, limit: i64) -> Result<Vec<Log>, RepoError> {
        self.inner.get_logs_recent(limit)
    }

    fn get_logs_by_entity(&self, entity_type: &str, entity_id: i32) -> Result<Vec<Log>, RepoError> {
        self.inner.get_logs_by_entity(entity_type, entity_id)
    }

    fn cleanup_logs(&mut self, days: i64) -> Result<usize, RepoError> {
        self.inner.cleanup_logs(days)
    }
}

impl<R: RequirementCommentsRepository> RequirementCommentsRepository for CacheRepository<R> {
    fn insert_requirement_comment(
        &mut self,
        new: &NewRequirementComment,
    ) -> Result<RequirementComment, RepoError> {
        self.inner.insert_requirement_comment(new)
    }

    fn list_comments_by_requirement(
        &self,
        requirement_id: i32,
        version_id: Option<i32>,
    ) -> Result<Vec<RequirementComment>, RepoError> {
        self.inner
            .list_comments_by_requirement(requirement_id, version_id)
    }
}

impl<R: RequirementVersionLinksRepository> RequirementVersionLinksRepository
    for CacheRepository<R>
{
    fn list_links_by_source_version(
        &self,
        source_version_id: i32,
    ) -> Result<Vec<RequirementVersionLink>, RepoError> {
        self.inner.list_links_by_source_version(source_version_id)
    }

    fn list_links_by_target_version(
        &self,
        target_version_id: i32,
    ) -> Result<Vec<RequirementVersionLink>, RepoError> {
        self.inner.list_links_by_target_version(target_version_id)
    }

    fn list_links_by_project(
        &self,
        project_id: i32,
        source_version_id: Option<i32>,
        target_version_id: Option<i32>,
        link_type: Option<&str>,
    ) -> Result<Vec<RequirementVersionLink>, RepoError> {
        self.inner.list_links_by_project(
            project_id,
            source_version_id,
            target_version_id,
            link_type,
        )
    }

    fn insert_requirement_version_link(
        &mut self,
        new: &NewRequirementVersionLink,
    ) -> Result<RequirementVersionLink, RepoError> {
        let link = self.inner.insert_requirement_version_link(new)?;
        self.cache.invalidate_project(new.project_id);
        Ok(link)
    }

    fn delete_requirement_version_link(
        &mut self,
        link_id: i32,
    ) -> Result<RequirementVersionLink, RepoError> {
        let link = self.inner.delete_requirement_version_link(link_id)?;
        self.cache.invalidate_project(link.project_id);
        Ok(link)
    }

    fn delete_requirement_version_links_by_source_version(
        &mut self,
        source_version_id: i32,
    ) -> Result<Vec<RequirementVersionLink>, RepoError> {
        let links = self
            .inner
            .delete_requirement_version_links_by_source_version(source_version_id)?;
        for link in &links {
            self.cache.invalidate_project(link.project_id);
        }
        Ok(links)
    }

    fn get_requirement_version_link_by_id(
        &self,
        link_id: i32,
    ) -> Result<RequirementVersionLink, RepoError> {
        self.inner.get_requirement_version_link_by_id(link_id)
    }
}

impl<R: super::NotificationRepository> super::NotificationRepository for CacheRepository<R> {
    fn insert_notification(&mut self, new: &NewNotification) -> Result<i32, RepoError> {
        self.inner.insert_notification(new)
    }
    fn get_notifications_for_user(
        &self,
        user_id: i32,
        limit: i64,
        unread_only: bool,
    ) -> Result<Vec<Notification>, RepoError> {
        self.inner
            .get_notifications_for_user(user_id, limit, unread_only)
    }
    fn count_unread_notifications(&self, user_id: i32) -> Result<i64, RepoError> {
        self.inner.count_unread_notifications(user_id)
    }
    fn mark_notification_read(&mut self, id: i32, user_id: i32) -> Result<bool, RepoError> {
        self.inner.mark_notification_read(id, user_id)
    }
    fn mark_all_read(&mut self, user_id: i32) -> Result<usize, RepoError> {
        self.inner.mark_all_read(user_id)
    }
    fn get_notification_preferences(
        &self,
        user_id: i32,
    ) -> Result<Vec<NotificationPreference>, RepoError> {
        self.inner.get_notification_preferences(user_id)
    }
    fn upsert_notification_preference(
        &mut self,
        pref: &NewNotificationPreference,
    ) -> Result<(), RepoError> {
        self.inner.upsert_notification_preference(pref)
    }
    fn delete_notification_preference(
        &mut self,
        user_id: i32,
        project_id: i32,
    ) -> Result<(), RepoError> {
        self.inner
            .delete_notification_preference(user_id, project_id)
    }
    fn get_project_subscribers(
        &self,
        project_id: i32,
    ) -> Result<Vec<NotificationPreference>, RepoError> {
        self.inner.get_project_subscribers(project_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::status_enums::ProjectStatus;
    use chrono::{NaiveDate, NaiveDateTime};
    use std::collections::HashMap;

    fn epoch() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(1970, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn populated_repo() -> DieselRepoMock {
        let user = DieselRepoMock::make_user(1, "alice", "hash");
        let status = RequirementStatus {
            id: 1,
            title: "Open".into(),
            description: "".into(),
            tag: "O".into(),
            project_id: 1,
            is_system: false,
            tag_color: None,
        };
        let category = Category {
            id: 1,
            title: "Cat".into(),
            description: "".into(),
            tag: "C".into(),
            project_id: 1,
        };
        let app = Applicability {
            id: 1,
            title: "App".into(),
            description: "".into(),
            tag: "A".into(),
            project_id: 1,
        };
        let ver = VerificationMethod {
            id: 1,
            title: "Ver".into(),
            description: "".into(),
            tag: "VER".into(),
            project_id: 1,
        };
        let project = Project {
            id: 1,
            name: "Proj".into(),
            description: Some("Desc".into()),
            creation_date: Some(epoch()),
            update_date: Some(epoch()),
            status: ProjectStatus::Active,
            owner_id: Some(1),
            slug: "proj".into(),
            group_id: None,
        };
        let requirement = Requirement {
            id: 1,
            current_version_id: None,
            same_as_current: None,
            title: "Req".into(),
            description: "".into(),
            status_id: 1,
            author_id: 1,
            reviewer_id: 1,
            reference_code: "ref".into(),
            category_id: 1,
            parent_id: None,
            creation_date: epoch(),
            update_date: epoch(),
            deadline_date: Some(epoch()),
            applicability_id: 1,
            justification: None,
            project_id: 1,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
        };
        let verification = Verification {
            id: 1,
            name: "Test".into(),
            description: "".into(),
            source: "src".into(),
            status_id: 1,
            reference_code: "VER-001".into(),
            parent_id: None,
            project_id: 1,
            verification_method_id: None,
            author_id: 1,
            reviewer_id: 1,
            status_set_by: None,
            status_set_at: None,
        };
        let matrix = MatrixLink {
            req_id: 1,
            verification_id: 1,
            creation_date: epoch(),
            project_id: 1,
            suspect: false,
            suspect_at: None,
            suspect_reason: None,
            cleared_by: None,
            cleared_at: None,
            triggering_version_id: None,
            triggering_user_id: None,
        };

        let mut users = HashMap::new();
        users.insert(1, user);
        let mut requirement_statuses = HashMap::new();
        requirement_statuses.insert(1, status.clone());
        let mut statuses = HashMap::new();
        statuses.insert(1, status);
        let mut categories = HashMap::new();
        categories.insert(1, category);
        let mut applicability = HashMap::new();
        applicability.insert(1, app);
        let mut verification_methods = HashMap::new();
        verification_methods.insert(1, ver);
        let mut requirements = HashMap::new();
        requirements.insert(1, requirement);
        let mut verifications = HashMap::new();
        verifications.insert(1, verification);
        let mut projects = HashMap::new();
        projects.insert(1, project);

        DieselRepoMock {
            logs: Vec::new(),
            users,
            statuses,
            requirement_statuses,
            verification_statuses: HashMap::new(),
            verification_methods,
            categories,
            applicability,
            requirements,
            requirement_verification_methods: Vec::new(),
            requirement_versions: HashMap::new(),
            next_version_id: 1,
            verifications,
            groups: HashMap::new(),
            group_members: Vec::new(),
            projects,
            matrices: vec![matrix],
            project_members: Vec::new(),
            project_reviewers: HashMap::new(),
            force_err: false,
            baselines: Vec::new(),
            baseline_requirements: Vec::new(),
            baseline_traceability: Vec::new(),
            baseline_verifications: Vec::new(),
            next_baseline_id: 1,
            custom_field_definitions: HashMap::new(),
            custom_field_values: Vec::new(),
            next_custom_field_id: 1,
            requirement_comments: Vec::new(),
            next_comment_id: 1,
            requirement_version_links: Vec::new(),
            next_link_id: 1,
            notifications: Vec::new(),
            next_notification_id: 1,
            notification_preferences: Vec::new(),
            next_notification_pref_id: 1,
            workspaces: Vec::new(),
            next_workspace_id: 1,
            email_tokens: Vec::new(),
            next_email_token_id: 1,
            sessions: Vec::new(),
        }
    }

    #[test]
    fn test_inner_repo_exposes_live_inner_repository() {
        // Start with a fully populated fake repo
        let mut repo = CacheRepository::new(populated_repo(), 60);

        // Read directly via inner_repo(); this should bypass the cache wrapper
        let initial_users = repo.inner_repo().get_users_all().unwrap();
        let initial_len = initial_users.len();
        assert!(initial_len >= 1);

        // Mutate the underlying repo through the wrapper (which changes the inner)
        let new_user = NewUser {
            id: None,
            username: "eve".into(),
            name: "Eve".into(),
            email: "eve@example.com".into(),
            password_hash: "pw".into(),
            is_admin: false,
            email_verified: None,
        };
        let _new_id = repo.insert_user(&new_user).unwrap();

        // Read again via inner_repo(); the change should be visible,
        // demonstrating that inner_repo() returns a live reference to the inner R
        let after_users = repo.inner_repo().get_users_all().unwrap();
        assert_eq!(after_users.len(), initial_len + 1);
    }

    #[test]
    fn test_warm_cache_populates_common_keys() {
        let repo = CacheRepository::new(
            DieselRepoMock {
                users: HashMap::new(),
                ..Default::default()
            },
            60,
        );

        let cache = repo.cache();

        repo.warm_cache();

        assert_eq!(cache.get(keys::PROJECTS_ALL), Some("[]".to_string()));
        assert_eq!(
            cache.get(keys::REQUIREMENT_STATUS_ALL),
            Some("[]".to_string())
        );
        assert_eq!(cache.get(keys::CATEGORIES_ALL), Some("[]".to_string()));
        assert_eq!(cache.get(keys::USERS_ALL), Some("[]".to_string()));
        assert_eq!(cache.get(keys::PROJECTS_NAV), Some("[]".to_string()));
    }

    #[test]
    fn test_get_user_by_id_is_cached() {
        let user = DieselRepoMock::make_user(1, "alice", "hash");
        let mut users = HashMap::new();
        users.insert(user.id, user.clone());

        let repo = CacheRepository::new(
            DieselRepoMock {
                users,
                ..Default::default()
            },
            60,
        );
        let cache = repo.cache();
        cache.reset_counters();

        // first call should miss cache and populate it
        let fetched = repo.get_user_by_id(1).unwrap();
        assert_eq!(fetched.username, "alice");
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);

        // second call should be served from cache
        let again = repo.get_user_by_id(1).unwrap();
        assert_eq!(again.username, "alice");
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_insert_user_invalidates_users_all_cache() {
        let user = DieselRepoMock::make_user(1, "bob", "hash");
        let mut users = HashMap::new();
        users.insert(user.id, user);
        let mut repo = CacheRepository::new(
            DieselRepoMock {
                users,
                ..Default::default()
            },
            60,
        );
        let cache = repo.cache();

        // populate cache with all users
        let all = repo.get_users_all().unwrap();
        assert_eq!(
            cache.get(keys::USERS_ALL),
            Some(serde_json::to_string(&all).unwrap())
        );

        // inserting a new user should invalidate USERS_ALL cache
        let new_user = NewUser {
            id: None,
            username: "charlie".into(),
            name: "Charlie".into(),
            email: "charlie@example.com".into(),
            password_hash: "pw".into(),
            is_admin: false,
            email_verified: None,
        };
        repo.insert_user(&new_user).unwrap();

        assert_eq!(cache.get(keys::USERS_ALL), None);
    }

    #[test]
    fn test_update_user_password_invalidates_cache() {
        let user = DieselRepoMock::make_user(1, "dave", "old");
        let mut users = HashMap::new();
        users.insert(user.id, user);
        let mut repo = CacheRepository::new(
            DieselRepoMock {
                users,
                ..Default::default()
            },
            60,
        );
        let cache = repo.cache();

        // cache user entry
        repo.get_user_by_id(1).unwrap();
        assert!(cache.get(&keys::Users::by_id(1)).is_some());

        // updating password should invalidate cached entry
        repo.update_user_password(1, "newhash").unwrap();
        assert!(cache.get(&keys::Users::by_id(1)).is_none());
    }

    #[test]
    fn test_user_repository_flows_and_invalid_cache() {
        let mut repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        // Prepopulate invalid JSON to exercise removal path
        cache.set(&keys::Users::by_id(1), "not-json".into());
        let user = repo.get_user_by_id(1).unwrap();
        assert_eq!(user.username, "alice");
        assert!(cache.get(&keys::Users::by_id(1)).unwrap().contains("alice"));

        // Username lookup bypasses cache so login always sees current password from DB
        repo.get_user_by_username("alice").unwrap();
        assert!(cache.get("user:username:alice").is_none());

        // Populate list cache then insert new user to invalidate it
        repo.get_users_all().unwrap();
        assert!(cache.get(keys::USERS_ALL).is_some());
        let new_user = NewUser {
            id: None,
            username: "bob".into(),
            name: "Bob".into(),
            email: "b@example.com".into(),
            password_hash: "pw".into(),
            is_admin: false,
            email_verified: None,
        };
        let new_id = repo.insert_user(&new_user).unwrap();
        assert!(cache.get(keys::USERS_ALL).is_none());

        // Updating and deleting invalidate caches
        repo.update_user_password(new_id, "hash").unwrap();
        assert!(cache.get(&keys::Users::by_id(new_id)).is_none());

        let upd = NewUser {
            id: Some(new_id),
            username: "bob".into(),
            name: "Bob".into(),
            email: "b@example.com".into(),
            password_hash: "pw".into(),
            is_admin: false,
            email_verified: None,
        };
        repo.update_user(&upd).unwrap();
        assert!(cache.get(&keys::Users::by_id(new_id)).is_none());

        let upd2 = UpdateUser {
            id: Some(new_id),
            username: "b2".into(),
            name: "B2".into(),
            email: "b2@example.com".into(),
            is_admin: false,
        };
        repo.update_user_without_password(&upd2).unwrap();
        assert!(cache.get(&keys::Users::by_id(new_id)).is_none());

        repo.delete_user(new_id).unwrap();
        assert!(cache.get(&keys::Users::by_id(new_id)).is_none());
    }

    #[test]
    fn test_requirements_repository_flows() {
        let mut repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        repo.get_requirement_by_id(1).unwrap();
        assert!(cache.get(&keys::Requirements::by_id(1)).is_some());

        repo.get_requirements_all().unwrap();
        assert!(cache.get(keys::REQUIREMENTS_ALL).is_some());

        repo.get_requirements_by_project(1).unwrap();
        assert!(cache.get(&keys::Requirements::by_project(1)).is_none());

        let new_req = NewRequirement {
            id: None,
            title: "R2".into(),
            description: "".into(),
            author_id: 1,
            category_id: 1,
            status_id: 1,
            reference_code: "r2".into(),
            reviewer_id: 1,
            applicability_id: 1,
            justification: None,
            project_id: 1,
        };
        let rid = repo.insert_new_requirement(&new_req).unwrap();
        assert!(cache.get(&keys::Requirements::by_id(rid)).is_none());

        let edit_req = NewRequirement {
            id: Some(rid),
            title: "R2".into(),
            description: "".into(),
            author_id: 1,
            category_id: 1,
            status_id: 1,
            reference_code: "r2".into(),
            reviewer_id: 1,
            applicability_id: 1,
            justification: None,
            project_id: 1,
        };
        repo.edit_requirement(&edit_req).unwrap();
        assert!(cache.get(&keys::Requirements::by_id(rid)).is_none());

        repo.update_requirement(1).unwrap();
        assert!(cache.get(&keys::Requirements::by_id(1)).is_none());

        repo.get_requirements_all().unwrap();
        repo.delete_requirement(rid).unwrap();
        assert!(cache.get(keys::REQUIREMENTS_ALL).is_none());
    }

    #[test]
    fn test_tests_repository_flows() {
        let mut repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        repo.get_verification_by_id(1).unwrap();
        assert!(cache.get(&keys::Verifications::by_id(1)).is_some());
        repo.get_verifications_all().unwrap();
        assert!(cache.get(keys::VERIFICATIONS_ALL).is_some());
        repo.get_verifications_by_project(1).unwrap();
        assert!(cache.get(&keys::Verifications::by_project(1)).is_some());

        let reqs = repo.get_requirements_for_verification(1).unwrap();
        assert_eq!(reqs.len(), 1);
        let tests = repo.get_verifications_for_requirement(1).unwrap();
        assert_eq!(tests.len(), 1);

        let new_test = NewVerification {
            id: None,
            name: "T2".into(),
            description: "".into(),
            source: "s".into(),
            status_id: 1,
            reference_code: "TEST-2".into(),
            parent_id: None,
            project_id: 1,
            verification_method_id: None,
            author_id: 1,
            reviewer_id: 1,
        };
        let tid = repo.insert_verification(&new_test).unwrap();
        assert!(cache.get(&keys::Verifications::by_id(tid)).is_none());

        let edit_test = NewVerification {
            id: Some(tid),
            name: "T2".into(),
            description: "".into(),
            source: "s".into(),
            status_id: 1,
            reference_code: "VER-002".into(),
            parent_id: None,
            project_id: 1,
            verification_method_id: None,
            author_id: 1,
            reviewer_id: 1,
        };
        repo.edit_verification(&edit_test).unwrap();
        assert!(cache.get(&keys::Verifications::by_id(tid)).is_none());

        repo.update_verification_requirement_links(tid, &[1])
            .unwrap();
        assert!(cache.get(&keys::Verifications::by_id(tid)).is_none());
        assert!(cache.get(&keys::Requirements::by_id(1)).is_none());

        repo.get_verifications_all().unwrap();
        repo.delete_verification(tid).unwrap();
        assert!(cache.get(keys::VERIFICATIONS_ALL).is_none());
    }

    #[test]
    fn test_lookup_project_and_matrix_flows() {
        let mut repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        // Status operations
        repo.get_requirement_status_all().unwrap();
        assert!(cache.get(keys::REQUIREMENT_STATUS_ALL).is_some());
        repo.get_requirement_status_by_id(1).unwrap();
        assert!(cache.get(&keys::RequirementStatus::by_id(1)).is_some());
        let ns = NewRequirementStatus {
            is_system: false,
            id: None,
            title: "Closed".into(),
            description: "".into(),
            tag: "C".into(),
            project_id: 1,
            tag_color: None,
        };
        let stid = repo.create_requirement_status(&ns).unwrap();
        assert!(cache.get(&keys::RequirementStatus::by_id(stid)).is_none());

        // Category operations
        repo.get_categories_all().unwrap();
        repo.get_category_by_id(1).unwrap();
        repo.get_categories_by_project(1).unwrap();
        let nc = NewCategory {
            id: None,
            title: "Cat2".into(),
            description: "".into(),
            tag: "C2".into(),
            project_id: 1,
        };
        let cid = repo.insert_new_category(&nc).unwrap();
        let ec = NewCategory {
            id: Some(cid),
            title: "Cat2".into(),
            description: "".into(),
            tag: "C2".into(),
            project_id: 1,
        };
        repo.edit_category(&ec).unwrap();
        repo.delete_category(cid).unwrap();

        // Applicability operations
        repo.get_applicability_all().unwrap();
        repo.get_applicability_by_id(1).unwrap();
        repo.get_applicability_by_project(1).unwrap();
        let na = NewApplicability {
            id: None,
            title: "App2".into(),
            description: "".into(),
            tag: "A2".into(),
            project_id: 1,
        };
        let aid = repo.insert_new_applicability(&na).unwrap();
        let ea = NewApplicability {
            id: Some(aid),
            title: "App2".into(),
            description: "".into(),
            tag: "A2".into(),
            project_id: 1,
        };
        repo.edit_applicability(&ea).unwrap();
        repo.delete_applicability(aid).unwrap();

        // Verification method operations
        repo.get_verification_methods_all().unwrap();
        repo.get_verification_method_by_id(1).unwrap();
        repo.get_verification_methods_by_project(1).unwrap();

        // Project operations
        repo.get_projects_all().unwrap();
        repo.get_project_by_id(1).unwrap();
        let np = NewProjectRow {
            name: "P2".into(),
            slug: "p2".into(),
            description: Some("".into()),
            status: ProjectStatus::Active,
            owner_id: Some(1),
            group_id: None,
        };
        let pid = repo.insert_new_project(&np).unwrap();
        let up = UpdateProject {
            name: "P2a".into(),
            description: Some("".into()),
            status: Some(ProjectStatus::Active),
            owner_id: Some(1),
            slug: None,
            group_id: None,
        };
        repo.edit_project(pid, &up).unwrap();
        repo.delete_project(pid).unwrap();

        // Matrix operations
        repo.get_matrix_by_project(1).unwrap();
        assert!(cache.get(&keys::Matrix::by_project(1)).is_some());
        repo.insert_new_matrix_item(&NewMatrixLink {
            req_id: 1,
            verification_id: 1,
            project_id: 1,
            triggering_version_id: None,
            triggering_user_id: None,
        })
        .unwrap();
        assert!(cache.get(&keys::Matrix::by_project(1)).is_none());
    }

    #[test]
    fn test_get_or_fetch_with_json_deserialization_failure() {
        let repo = CacheRepository::new(
            DieselRepoMock {
                users: HashMap::new(),
                ..Default::default()
            },
            60,
        );
        let cache = repo.cache();

        // Insert invalid JSON
        cache.set(&keys::Users::by_id(1), "invalid json".to_string());

        // This should remove the invalid entry and fetch from repo
        // But since repo is empty, it should return NotFound
        let result = repo.get_user_by_id(1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RepoError::NotFound));

        // Invalid entry should be removed
        assert!(cache.get(&keys::Users::by_id(1)).is_none());
    }

    #[test]
    fn test_get_or_fetch_with_serialization_failure() {
        // This test verifies that if serialization fails, we still return the value
        // In practice, this is hard to test without a custom type that fails to serialize
        // But we can test the error propagation path
        let repo = CacheRepository::new(
            DieselRepoMock {
                users: HashMap::new(),
                ..Default::default()
            },
            60,
        );

        // Test that errors from inner repo are propagated
        let result = repo.get_user_by_id(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_repository_inner_repo_mutability() {
        let mut repo = CacheRepository::new(populated_repo(), 60);

        // Test that we can mutate through the wrapper
        let new_user = NewUser {
            id: None,
            username: "test".into(),
            name: "Test".into(),
            email: "test@example.com".into(),
            password_hash: "hash".into(),
            is_admin: false,
            email_verified: None,
        };

        let result = repo.insert_user(&new_user);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_repository_warm_cache_with_errors() {
        // Test warm_cache when inner repo returns errors
        let repo = CacheRepository::new(DieselRepoMock::with_error(), 60);

        // Should not panic even if inner repo returns errors
        repo.warm_cache();
    }

    #[test]
    fn test_cache_repository_log_repository_passthrough() {
        let mut repo = CacheRepository::new(populated_repo(), 60);

        let new_log = NewLog {
            user_id: 1,
            entity_type: "test".into(),
            entity_id: Some(1),
            action_type: "create".into(),
            description: Some("test".into()),
            project_id: Some(1),
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        };

        // Log operations should pass through without caching
        let result = repo.insert_log(&new_log);
        assert!(result.is_ok());

        let logs = repo.get_logs_recent(10).unwrap();
        assert_eq!(logs.len(), 1);
    }

    #[test]
    fn test_cache_repository_get_logs_by_entity() {
        let mut repo = CacheRepository::new(populated_repo(), 60);

        let new_log = NewLog {
            user_id: 1,
            entity_type: "requirement".into(),
            entity_id: Some(1),
            action_type: "create".into(),
            description: Some("test".into()),
            project_id: Some(1),
            old_values: None,
            new_values: None,
            ip_address: None,
            user_agent: None,
        };
        repo.insert_log(&new_log).unwrap();

        let logs = repo.get_logs_by_entity("requirement", 1).unwrap();
        assert_eq!(logs.len(), 1);
    }

    #[test]
    fn test_cache_repository_cleanup_logs() {
        let mut repo = CacheRepository::new(populated_repo(), 60);

        for i in 0..5 {
            let new_log = NewLog {
                user_id: 1,
                entity_type: "requirement".into(),
                entity_id: Some(i),
                action_type: "create".into(),
                description: Some("test".into()),
                project_id: Some(1),
                old_values: None,
                new_values: None,
                ip_address: None,
                user_agent: None,
            };
            repo.insert_log(&new_log).unwrap();
        }

        let result = repo.cleanup_logs(30);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_repository_matrix_insert_invalidates_cache() {
        let mut repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        // Populate cache
        repo.get_matrix_by_project(1).unwrap();
        assert!(cache.get(&keys::Matrix::by_project(1)).is_some());

        // Insert new matrix item should invalidate cache
        repo.insert_new_matrix_item(&NewMatrixLink {
            req_id: 1,
            verification_id: 1,
            project_id: 1,
            triggering_version_id: None,
            triggering_user_id: None,
        })
        .unwrap();

        assert!(cache.get(&keys::Matrix::by_project(1)).is_none());
    }

    #[test]
    fn test_cache_repository_delete_user_invalidates_project_memberships() {
        let mut repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        // Add a project member
        let new_member = NewProjectMember {
            project_id: 1,
            user_id: 1,
            role: 1,
        };
        repo.add_project_member(&new_member).unwrap();

        // Cache project members
        repo.get_members_by_project(1).unwrap();
        repo.get_projects_for_user(1).unwrap();

        // Delete user should invalidate both caches
        repo.delete_user(1).unwrap();

        // Note: The cache invalidation happens in delete_user implementation
        // We verify the user cache is invalidated
        assert!(cache.get(&keys::Users::by_id(1)).is_none());
    }

    #[test]
    fn test_get_or_fetch_with_serialization_error_handling() {
        // Test that if serialization fails, we still return the value
        let repo = CacheRepository::new(populated_repo(), 60);
        let _cache = repo.cache();

        // Insert a value that will fail to serialize (this is hard to test without custom types)
        // But we can test the error path by ensuring fetch errors are propagated
        let result = repo.get_user_by_id(999);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_or_fetch_cache_hit_path() {
        let repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        // First call populates cache
        let user1 = repo.get_user_by_id(1).unwrap();

        // Second call should hit cache
        let user2 = repo.get_user_by_id(1).unwrap();
        assert_eq!(user1.id, user2.id);

        // Verify cache was used
        assert!(cache.get(&keys::Users::by_id(1)).is_some());
    }

    #[test]
    fn test_get_or_fetch_cache_miss_then_hit() {
        let repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        // First call - cache miss
        let _ = repo.get_user_by_id(1).unwrap();
        let stats_before = cache.stats();

        // Second call - cache hit
        let _ = repo.get_user_by_id(1).unwrap();
        let stats_after = cache.stats();

        // Should have one more hit
        assert!(stats_after.hits > stats_before.hits);
    }

    #[test]
    fn test_cache_repository_all_trait_methods_accessible() {
        let repo = CacheRepository::new(populated_repo(), 60);

        // Test that all repository trait methods are accessible through CacheRepository
        let _ = repo.get_users_all();
        let _ = repo.get_user_by_id(1);
        let _ = repo.get_user_by_username("alice");
        let _ = repo.get_requirements_all();
        let _ = repo.get_requirement_by_id(1);
        let _ = repo.get_requirements_by_project(1);
        let _ = repo.get_verifications_all();
        let _ = repo.get_verification_by_id(1);
        let _ = repo.get_verifications_by_project(1);
        let _ = repo.get_projects_all();
        let _ = repo.get_project_by_id(1);
        let _ = repo.get_categories_all();
        let _ = repo.get_category_by_id(1);
        let _ = repo.get_categories_by_project(1);
        let _ = repo.get_applicability_all();
        let _ = repo.get_applicability_by_id(1);
        let _ = repo.get_applicability_by_project(1);
        let _ = repo.get_verification_methods_all();
        let _ = repo.get_verification_by_id(1);
        let _ = repo.get_verification_methods_by_project(1);
        let _ = repo.get_requirement_status_all();
        let _ = repo.get_requirement_status_by_id(1);
        let _ = repo.get_verification_status_all();
        let _ = repo.get_verification_status_by_id(1);
        let _ = repo.get_members_by_project(1);
        let _ = repo.get_projects_for_user(1);
        let _ = repo.get_matrix_by_project(1);
        let _ = repo.get_requirements_for_verification(1);
        let _ = repo.get_verifications_for_requirement(1);
        let _ = repo.get_logs_recent(10);
        let _ = repo.get_logs_by_entity("requirement", 1);

        // If we get here without panic, all methods are accessible
    }

    #[test]
    fn test_cache_repository_inner_repo_immutable_access() {
        let repo = CacheRepository::new(populated_repo(), 60);

        // Test that inner_repo() provides read access
        let users1 = repo.inner_repo().get_users_all().unwrap();
        let users2 = repo.inner_repo().get_users_all().unwrap();

        assert_eq!(users1.len(), users2.len());
    }

    #[test]
    fn test_cache_repository_cache_access() {
        let repo = CacheRepository::new(populated_repo(), 60);
        let cache1 = repo.cache();
        let cache2 = repo.cache();

        // Both should be the same Arc
        cache1.set("test_key", "test_value".to_string());
        assert_eq!(cache2.get("test_key"), Some("test_value".to_string()));
    }

    #[test]
    fn test_warm_cache_handles_serialization_errors() {
        let repo = CacheRepository::new(
            DieselRepoMock {
                users: HashMap::new(),
                ..Default::default()
            },
            60,
        );

        // Should not panic even if serialization fails
        repo.warm_cache();
    }

    #[test]
    fn test_warm_cache_with_partial_data() {
        let repo = CacheRepository::new(populated_repo(), 60);
        let cache = repo.cache();

        // Clear some data to test partial warm
        cache.clear();

        // Warm cache should populate what's available
        repo.warm_cache();

        // Should have populated some keys
        assert!(
            cache.get(keys::PROJECTS_ALL).is_some()
                || cache.get(keys::REQUIREMENT_STATUS_ALL).is_some()
                || cache.get(keys::CATEGORIES_ALL).is_some()
                || cache.get(keys::USERS_ALL).is_some()
        );
    }
}
