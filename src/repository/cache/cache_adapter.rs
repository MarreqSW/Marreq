// NOTE: This is a temporal Adapter (for DieselRepo)until the
// Middleware refactor with Repository Errors is completed.

use crate::helper_functions::decorators::decorate_tests;
use crate::models::*;
use crate::repository::errors::RepoError;
use crate::repository::{
    DieselCachedRepo, LookupRepository, Cache, MatrixRepository, ProjectsRepository,
    RequirementsRepository, TestsRepository, UserRepository,
};
use chrono;
use std::sync::Arc;

/// Get projects for navigation with caching
pub fn get_projects_for_nav_cached() -> Result<Vec<Project>, String> {
    DieselCachedRepo::read()
        .get_projects_all()
        .map_err(|e| e.to_string())
}

/// Get all statuses with caching
pub fn get_status_all_cached() -> Result<Vec<Status>, String> {
    DieselCachedRepo::read()
        .get_status_all()
        .map_err(|e| e.to_string())
}

/// Get all categories with caching
pub fn get_categories_all_cached() -> Result<Vec<Category>, String> {
    DieselCachedRepo::read()
        .get_categories_all()
        .map_err(|e| e.to_string())
}

/// Get all applicability with caching
pub fn get_applicability_all_cached() -> Result<Vec<Applicability>, String> {
    DieselCachedRepo::read()
        .get_applicability_all()
        .map_err(|e| e.to_string())
}

/// Get all verification data with caching
pub fn get_verification_all_cached() -> Result<Vec<Verification>, String> {
    DieselCachedRepo::read()
        .get_verification_all()
        .map_err(|e| e.to_string())
}

/// Get all users with caching
pub fn get_users_all_cached() -> Result<Vec<User>, String> {
    DieselCachedRepo::read()
        .get_users_all()
        .map_err(|e| e.to_string())
}

/// Get user by ID with caching
pub fn get_user_by_id_cached(id: i32) -> User {
    DieselCachedRepo::read()
        .get_user_by_id(id)
        .expect("Error reading table Users")
}

/// Get requirements by project with caching
pub fn get_requirements_by_project_cached(project_id: i32) -> Result<Vec<Requirement>, String> {
    // Cache for 5 minutes
    DieselCachedRepo::read()
        .get_requirements_by_project(project_id)
        .map_err(|e| e.to_string())
}

/// Get tests by project with caching
pub fn get_tests_by_project_cached(project_id: i32) -> Result<Vec<Test>, String> {
    // Cache for 5 minutes
    DieselCachedRepo::read()
        .get_tests_by_project(project_id)
        .map_err(|e| e.to_string())
}

/// Get matrix by project with caching
pub fn get_matrix_by_project_cached(project_id: i32) -> Result<Vec<Matrix>, String> {
    // Cache for 3 minutes (matrix data is more dynamic)
    DieselCachedRepo::read()
        .get_matrix_by_project(project_id)
        .map_err(|e| e.to_string())
}

/// Get requirement by ID with caching
pub fn get_requirement_by_id_cached(id: i32) -> Requirement {
    let fallback = || Requirement {
        req_id: id,
        req_title: format!("Unknown Requirement ({})", id),
        req_description: "Requirement not found".to_string(),
        req_verification: 1,
        req_current_status: 1,
        req_author: 1,
        req_reviewer: 1,
        req_link: "".to_string(),
        req_reference: format!("REQ-UNK-{}", id),
        req_category: 1,
        req_parent: 0,
        req_creation_date: chrono::Utc::now().naive_utc(),
        req_update_date: chrono::Utc::now().naive_utc(),
        req_deadline_date: chrono::Utc::now().naive_utc(),
        req_applicability: 1,
        req_justification: None,
        project_id: 1,
    };

    DieselCachedRepo::read()
        .get_requirement_by_id(id)
        .unwrap_or_else(|_| fallback())
}

/// Get requirement by ID with caching and proper error handling
pub fn get_requirement_by_id_cached_safe(id: i32) -> Result<Requirement, String> {
    DieselCachedRepo::read()
        .get_requirement_by_id(id)
        .map_err(|e| match e {
            RepoError::NotFound => format!("Requirement with ID {} not found", id),
            _ => e.to_string(),
        })
}

/// Get test by ID with caching
pub fn get_test_by_id_cached(id: i32) -> Test {
    DieselCachedRepo::read()
        .get_test_by_id(id)
        .expect("Error reading table Tests")
}

/// Get test by ID with caching and proper error handling
pub fn get_test_by_id_cached_safe(id: i32) -> Result<Test, String> {
    DieselCachedRepo::read()
        .get_test_by_id(id)
        .map_err(|e| match e {
            RepoError::NotFound => format!("Test with ID {} not found", id),
            _ => e.to_string(),
        })
}

/// Get category by ID with caching
pub fn get_category_by_id_cached(id: i32) -> Category {
    let fallback = || Category {
        cat_id: id,
        cat_title: format!("Unknown Category ({})", id),
        cat_description: "Category not found".to_string(),
        cat_tag: "unknown".to_string(),
        project_id: 1,
    };
    DieselCachedRepo::read()
        .get_category_by_id(id)
        .unwrap_or_else(|_| fallback())
}

/// Cached version of get_requirements_all with project filtering
pub fn get_requirements_all_cached() -> Result<Vec<Requirement>, String> {
    DieselCachedRepo::read()
        .get_requirements_all()
        .map_err(|e| e.to_string())
}

/// Cached version of get_tests_all with project filtering
pub fn get_tests_all_cached() -> Result<Vec<Test>, String> {
    DieselCachedRepo::read()
        .get_tests_all()
        .map_err(|e| e.to_string())
}

/// Invalidate cache when requirements are modified
pub fn invalidate_requirement_cache_complete(req_id: i32) {
    DieselCachedRepo::write().cache().invalidate_requirement(req_id);
    // Also invalidate project-level caches
    // Note: In a real implementation, you'd need to track which project the requirement belongs to
}

/// Invalidate cache when tests are modified
pub fn invalidate_test_cache_complete(test_id: i32) {
    DieselCachedRepo::write().cache().invalidate_test(test_id);
    // Also invalidate project-level caches
    // Note: In a real implementation, you'd need to track which project the test belongs to
}

/// Invalidate cache when users are modified
pub fn invalidate_user_cache_complete(user_id: i32) {
    DieselCachedRepo::write().cache().invalidate_user(user_id);
}

/// Invalidate cache when categories are modified
pub fn invalidate_category_cache_complete(cat_id: i32) {
    DieselCachedRepo::write().cache().invalidate_category(cat_id);
}

/// Invalidate cache when projects are modified
pub fn invalidate_project_cache_complete(project_id: i32) {
    DieselCachedRepo::write().cache().invalidate_project(project_id);
}

/// Invalidate cache when applicability is modified
pub fn invalidate_applicability_cache_complete(applicability_id: i32) {
    DieselCachedRepo::write().cache().invalidate_applicability(applicability_id);
}

/// Get verification by project with caching
pub fn get_verification_by_project_cached(project_id: i32) -> Result<Vec<Verification>, String> {
    DieselCachedRepo::read()
        .get_verification_by_project(project_id)
        .map_err(|e| e.to_string())
}

/// Get categories by project with caching
pub fn get_categories_by_project_cached(project_id: i32) -> Result<Vec<Category>, String> {
    DieselCachedRepo::read()
        .get_categories_by_project(project_id)
        .map_err(|e| e.to_string())
}

/// Get applicability by project with caching
pub fn get_applicability_by_project_cached(project_id: i32) -> Result<Vec<Applicability>, String> {
    DieselCachedRepo::read()
        .get_applicability_by_project(project_id)
        .map_err(|e| e.to_string())
}

/// Get linked tests for requirement with caching
pub fn get_linked_tests_for_requirement_cached(req_id: i32) -> Result<Vec<DecoratedTest>, String> {
    DieselCachedRepo::read()
        .get_tests_for_requirement(req_id)
        .map(|tests| decorate_tests(tests))
        .map_err(|e| e.to_string())
}

/// Get requirements for test with caching
pub fn get_requirements_for_test_cached(test_id: i32) -> Result<Vec<Requirement>, String> {
    DieselCachedRepo::read()
        .get_requirements_for_test(test_id)
        .map_err(|e| e.to_string())
}

/// Get applicability by ID with caching
pub fn get_applicability_by_id_cached(id: i32) -> Applicability {
    let fallback = || Applicability {
        app_id: id,
        app_title: format!("Unknown Applicability ({})", id),
        app_description: "Applicability not found".to_string(),
        app_tag: "unknown".to_string(),
        project_id: 1,
    };
    DieselCachedRepo::read()
        .get_applicability_by_id(id)
        .unwrap_or_else(|_| fallback())
}

/// Get project by ID with caching
pub fn get_project_by_id_cached(project_id: i32) -> Project {
    DieselCachedRepo::read()
        .get_project_by_id(project_id)
        .expect("Error loading project")
}

/// Get status name by ID with caching
pub fn get_status_name_by_id_cached(id: i32) -> String {
    DieselCachedRepo::read()
        .get_status_by_id(id)
        .map(|s| s.st_title)
        .unwrap_or_else(|_| "[Status Not Found]".to_string())
}

/// Get all projects with caching
pub fn get_projects_all_cached() -> Result<Vec<Project>, String> {
    DieselCachedRepo::read()
        .get_projects_all()
        .map_err(|e| e.to_string())
}


/// Cache utility functions
///
/// The application maintains a shared [`CacheRepository`] instance that owns the
/// cache. This helper provides a convenient way to access that cache without
/// exposing a global mutable state.
pub fn get_cache() -> Arc<Cache> {
    crate::repository::diesel_repo::DieselCachedRepo::read().cache()
}


/// Invalidate all cache entries (use with caution)
pub fn invalidate_all_cache() {
    get_cache().clear();
}

/// Warm up the cache with frequently accessed data
// VPR: I discourage this function as it copies the whole DB!
pub fn warm_cache() {
    use crate::repository::DieselRepo;
    use crate::repository::keys;
    use std::time::Duration;

    let cache = get_cache();

    let repo = DieselRepo::new();

    // Warm up projects cache
    if let Ok(projects) = repo.get_projects_all() {
        if let Ok(json_data) = serde_json::to_string(&projects) {
            cache.set_with_ttl(keys::PROJECTS_ALL, json_data, Duration::from_secs(600));
        }
    }

    // Warm up status cache
    if let Ok(statuses) = repo.get_status_all() {
        if let Ok(json_data) = serde_json::to_string(&statuses) {
            cache.set_with_ttl(keys::STATUS_ALL, json_data, Duration::from_secs(900));
        }
    }

    // Warm up categories cache
    if let Ok(categories) = repo.get_categories_all() {
        if let Ok(json_data) = serde_json::to_string(&categories) {
            cache.set_with_ttl(keys::CATEGORIES_ALL, json_data, Duration::from_secs(900));
        }
    }

    // Warm up users cache
    if let Ok(users) = repo.get_users_all() {
        if let Ok(json_data) = serde_json::to_string(&users) {
            cache.set_with_ttl(keys::USERS_ALL, json_data, Duration::from_secs(600));
        }
    }

    // Warm up projects navigation cache
    if let Ok(projects) = repo.get_projects_all() {
        if let Ok(json_data) = serde_json::to_string(&projects) {
            cache.set_with_ttl(keys::PROJECTS_NAV, json_data, Duration::from_secs(300));
        }
    }
}
