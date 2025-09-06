use crate::cache::{
    invalidate_category_cache, invalidate_project_cache, invalidate_requirement_cache,
    invalidate_test_cache, invalidate_user_cache,
};
use crate::helper_functions::decorators::decorate_tests;
use crate::models::*;
use crate::repository::errors::RepoError;
use crate::repository::{
    CacheRepository, LookupRepository, MatrixRepository, ProjectsRepository,
    RequirementsRepository, TestsRepository, UserRepository,
};
use chrono;

/// Get projects for navigation with caching
pub fn get_projects_for_nav_cached() -> Result<Vec<Project>, String> {
    CacheRepository::new()
        .get_projects_all()
        .map_err(|e| e.to_string())
}

/// Get all statuses with caching
pub fn get_status_all_cached() -> Result<Vec<Status>, String> {
    CacheRepository::new()
        .get_status_all()
        .map_err(|e| e.to_string())
}

/// Get all categories with caching
pub fn get_categories_all_cached() -> Result<Vec<Category>, String> {
    CacheRepository::new()
        .get_categories_all()
        .map_err(|e| e.to_string())
}

/// Get all applicability with caching
pub fn get_applicability_all_cached() -> Result<Vec<Applicability>, String> {
    CacheRepository::new()
        .get_applicability_all()
        .map_err(|e| e.to_string())
}

/// Get all verification data with caching
pub fn get_verification_all_cached() -> Result<Vec<Verification>, String> {
    CacheRepository::new()
        .get_verification_all()
        .map_err(|e| e.to_string())
}

/// Get all users with caching
pub fn get_users_all_cached() -> Result<Vec<User>, String> {
    CacheRepository::new()
        .get_users_all()
        .map_err(|e| e.to_string())
}

/// Get user by ID with caching
pub fn get_user_by_id_cached(id: i32) -> User {
    CacheRepository::new()
        .get_user_by_id(id)
        .expect("Error reading table Users")
}

/// Get requirements by project with caching
pub fn get_requirements_by_project_cached(project_id: i32) -> Result<Vec<Requirement>, String> {
    // Cache for 5 minutes
    CacheRepository::new()
        .get_requirements_by_project(project_id)
        .map_err(|e| e.to_string())
}

/// Get tests by project with caching
pub fn get_tests_by_project_cached(project_id: i32) -> Result<Vec<Test>, String> {
    // Cache for 5 minutes
    CacheRepository::new()
        .get_tests_by_project(project_id)
        .map_err(|e| e.to_string())
}

/// Get matrix by project with caching
pub fn get_matrix_by_project_cached(project_id: i32) -> Result<Vec<Matrix>, String> {
    // Cache for 3 minutes (matrix data is more dynamic)
    CacheRepository::new()
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

    CacheRepository::new()
        .get_requirement_by_id(id)
        .unwrap_or_else(|_| fallback())
}

/// Get requirement by ID with caching and proper error handling
pub fn get_requirement_by_id_cached_safe(id: i32) -> Result<Requirement, String> {
    CacheRepository::new()
        .get_requirement_by_id(id)
        .map_err(|e| match e {
            RepoError::NotFound => format!("Requirement with ID {} not found", id),
            _ => e.to_string(),
        })
}

/// Get test by ID with caching
pub fn get_test_by_id_cached(id: i32) -> Test {
    CacheRepository::new()
        .get_test_by_id(id)
        .expect("Error reading table Tests")
}

/// Get test by ID with caching and proper error handling
pub fn get_test_by_id_cached_safe(id: i32) -> Result<Test, String> {
    CacheRepository::new()
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
    CacheRepository::new()
        .get_category_by_id(id)
        .unwrap_or_else(|_| fallback())
}

/// Cached version of get_requirements_all with project filtering
pub fn get_requirements_all_cached() -> Result<Vec<Requirement>, String> {
    CacheRepository::new()
        .get_requirements_all()
        .map_err(|e| e.to_string())
}

/// Cached version of get_tests_all with project filtering
pub fn get_tests_all_cached() -> Result<Vec<Test>, String> {
    CacheRepository::new()
        .get_tests_all()
        .map_err(|e| e.to_string())
}

/// Invalidate cache when requirements are modified
pub fn invalidate_requirement_cache_complete(req_id: i32) {
    invalidate_requirement_cache(req_id);
    // Also invalidate project-level caches
    // Note: In a real implementation, you'd need to track which project the requirement belongs to
}

/// Invalidate cache when tests are modified
pub fn invalidate_test_cache_complete(test_id: i32) {
    invalidate_test_cache(test_id);
    // Also invalidate project-level caches
    // Note: In a real implementation, you'd need to track which project the test belongs to
}

/// Invalidate cache when users are modified
pub fn invalidate_user_cache_complete(user_id: i32) {
    invalidate_user_cache(user_id);
}

/// Invalidate cache when categories are modified
pub fn invalidate_category_cache_complete(cat_id: i32) {
    invalidate_category_cache(cat_id);
}

/// Invalidate cache when projects are modified
pub fn invalidate_project_cache_complete(project_id: i32) {
    invalidate_project_cache(project_id);
}

/// Invalidate cache when applicability is modified
pub fn invalidate_applicability_cache_complete(applicability_id: i32) {
    crate::cache::invalidate_applicability_cache(applicability_id);
}

/// Get verification by project with caching
pub fn get_verification_by_project_cached(project_id: i32) -> Result<Vec<Verification>, String> {
    CacheRepository::new()
        .get_verification_by_project(project_id)
        .map_err(|e| e.to_string())
}

/// Get categories by project with caching
pub fn get_categories_by_project_cached(project_id: i32) -> Result<Vec<Category>, String> {
    CacheRepository::new()
        .get_categories_by_project(project_id)
        .map_err(|e| e.to_string())
}

/// Get applicability by project with caching
pub fn get_applicability_by_project_cached(project_id: i32) -> Result<Vec<Applicability>, String> {
    CacheRepository::new()
        .get_applicability_by_project(project_id)
        .map_err(|e| e.to_string())
}

/// Get linked tests for requirement with caching
pub fn get_linked_tests_for_requirement_cached(
    req_id: i32,
) -> Result<Vec<DecoratedTest>, String> {
    CacheRepository::new()
        .get_tests_for_requirement(req_id)
        .map(|tests| decorate_tests(tests))
        .map_err(|e| e.to_string())
}

/// Get requirements for test with caching
pub fn get_requirements_for_test_cached(test_id: i32) -> Result<Vec<Requirement>, String> {
    CacheRepository::new()
        .get_requirements_for_test(test_id)
        .map_err(|e| e.to_string())
}

/// Get status by ID with caching
pub fn get_status_by_id_cached(id: i32) -> Status {
    CacheRepository::new()
        .get_status_by_id(id)
        .expect("Error reading table Status")
}

/// Get verification by ID with caching
pub fn get_verification_by_id_cached(id: i32) -> Verification {
    let fallback = || Verification {
        verification_id: id,
        verification_name: format!("Unknown Verification ({})", id),
        verification_description: "Verification not found".to_string(),
        project_id: 1,
    };
    CacheRepository::new()
        .get_verification_by_id(id)
        .unwrap_or_else(|_| fallback())
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
    CacheRepository::new()
        .get_applicability_by_id(id)
        .unwrap_or_else(|_| fallback())
}

/// Get project by ID with caching
pub fn get_project_by_id_cached(project_id: i32) -> Project {
    CacheRepository::new()
        .get_project_by_id(project_id)
        .expect("Error loading project")
}

/// Get requirement title by ID with caching
pub fn get_requirement_title_by_id_cached(id: i32) -> String {
    CacheRepository::new()
        .get_requirement_by_id(id)
        .map(|r| r.req_title)
        .unwrap_or_else(|_| "[Requirement Not Found]".to_string())
}

/// Get test status by ID with caching
pub fn get_test_status_by_id_cached(id: i32) -> String {
    let repo = CacheRepository::new();
    let status = if let Ok(test) = repo.get_test_by_id(id) {
        repo.get_status_by_id(test.test_status)
            .map(|s| s.st_title)
            .unwrap_or_else(|_| "[Status Not Found]".to_string())
    } else {
        "[Test Not Found]".to_string()
    };
    status
}

/// Get status name by ID with caching
pub fn get_status_name_by_id_cached(id: i32) -> String {
    CacheRepository::new()
        .get_status_by_id(id)
        .map(|s| s.st_title)
        .unwrap_or_else(|_| "[Status Not Found]".to_string())
}

/// Get all projects with caching
pub fn get_projects_all_cached() -> Result<Vec<Project>, String> {
    CacheRepository::new()
        .get_projects_all()
        .map_err(|e| e.to_string())
}

/* TODO: never used ??
use std::time::Duration;

/// Warm up frequently accessed cache entries
pub fn warm_cache() -> Result<(), String> {
    // Warm up navigation data
    let _ = get_projects_for_nav_cached();

    // Warm up status and category data
    let _ = get_status_all_cached();
    let _ = get_categories_all_cached();

    // Warm up applicability and verification data
    let _ = get_applicability_all_cached();
    let _ = get_verification_all_cached();

    // Warm up users data
    let _ = get_users_all_cached();

    Ok(())
}

/// Get cache statistics with additional metrics
pub fn get_cache_stats_extended() -> Result<serde_json::Value, String> {
    let cache = get_cache();
    let stats = cache.stats();

    // Get some sample keys to show what's cached
    let sample_keys = vec![
        keys::PROJECTS_NAV,
        keys::STATUS_ALL,
        keys::CATEGORIES_ALL,
        keys::APPLICABILITY_ALL,
        keys::VERIFICATION_ALL,
        keys::USERS_ALL,
    ];

    let mut key_status = serde_json::Map::new();
    for key in sample_keys {
        let exists = cache.get(key).is_some();
        key_status.insert(key.to_string(), serde_json::Value::Bool(exists));
    }

    Ok(json!({
        "stats": stats,
        "key_status": key_status,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Bulk invalidate cache for multiple entities
pub fn bulk_invalidate_cache(entity_type: &str, entity_ids: &[i32]) {
    let cache = get_cache();

    match entity_type {
        "requirement" => {
            for &id in entity_ids {
                invalidate_requirement_cache(id);
            }
            // Also invalidate project-specific caches
            if let Ok(projects) = CacheRepository::new().get_projects_all() {
                for project in projects {
                    cache.remove(&keys::Requirements::by_project(project.project_id));
                }
            }
        }
        "test" => {
            for &id in entity_ids {
                invalidate_test_cache(id);
            }
            // Also invalidate project-specific caches
            if let Ok(projects) = CacheRepository::new().get_projects_all() {
                for project in projects {
                    cache.remove(&keys::Tests::by_project(project.project_id));
                }
            }
        }
        "category" => {
            for &id in entity_ids {
                invalidate_category_cache(id);
            }
            cache.remove(keys::CATEGORIES_ALL);
        }
        "user" => {
            for &id in entity_ids {
                invalidate_user_cache(id);
            }
            cache.remove(keys::USERS_ALL);
        }
        "project" => {
            for &id in entity_ids {
                invalidate_project_cache(id);
            }
            cache.remove(keys::PROJECTS_ALL);
            cache.remove(keys::PROJECTS_NAV);
        }
        _ => {
            // Unknown entity type, invalidate all caches as fallback
            crate::cache::invalidate_all_cache();
        }
    }
}

/// Smart cache invalidation based on entity relationships
pub fn smart_invalidate_cache(entity_type: &str, entity_id: i32, related_entities: &[(String, i32)]) {
    // First, invalidate the main entity
    match entity_type {
        "requirement" => invalidate_requirement_cache(entity_id),
        "test" => invalidate_test_cache(entity_id),
        "category" => invalidate_category_cache(entity_id),
        "user" => invalidate_user_cache(entity_id),
        "project" => invalidate_project_cache(entity_id),
        _ => {}
    }

    // Then invalidate related entities
    for (related_type, related_id) in related_entities {
        match related_type.as_str() {
            "requirement" => invalidate_requirement_cache(*related_id),
            "test" => invalidate_test_cache(*related_id),
            "category" => invalidate_category_cache(*related_id),
            "user" => invalidate_user_cache(*related_id),
            "project" => invalidate_project_cache(*related_id),
            _ => {}
        }
    }

    // Finally, invalidate aggregate caches that might be affected
    let cache = get_cache();
    match entity_type {
        "requirement" | "test" => {
            // Invalidate project-specific caches
            if let Ok(projects) = CacheRepository::new().get_projects_all() {
                for project in projects {
                    cache.remove(&keys::Requirements::by_project(project.project_id));
                    cache.remove(&keys::Tests::by_project(project.project_id));
                }
            }
        }
        "category" => {
            cache.remove(keys::CATEGORIES_ALL);
            cache.remove(keys::APPLICABILITY_ALL);
        }
        "user" => {
            cache.remove(keys::USERS_ALL);
        }
        "project" => {
            cache.remove(keys::PROJECTS_ALL);
            cache.remove(keys::PROJECTS_NAV);
        }
        _ => {}
    }
}

/// Cache warming for specific project data
pub fn warm_project_cache(project_id: i32) {
    let cache = get_cache();

    // Warm up project-specific requirements
    if let Ok(requirements) = CacheRepository::new().get_requirements_by_project(project_id) {
        if let Ok(json_data) = serde_json::to_string(&requirements) {
            cache.set_with_ttl(
                &keys::Requirements::by_project(project_id),
                json_data,
                Duration::from_secs(300)
            );
        }
    }

    // Warm up project-specific tests
    if let Ok(tests) = CacheRepository::new().get_tests_by_project(project_id) {
        if let Ok(json_data) = serde_json::to_string(&tests) {
            cache.set_with_ttl(
                &keys::Tests::by_project(project_id),
                json_data,
                Duration::from_secs(300)
            );
        }
    }

    // Warm up project-specific categories
    if let Ok(categories) = CacheRepository::new().get_categories_by_project(project_id) {
        if let Ok(json_data) = serde_json::to_string(&categories) {
            cache.set_with_ttl(
                &keys::Categories::by_project(project_id),
                json_data,
                Duration::from_secs(600)
            );
        }
    }

    // Warm up project-specific verification types
    if let Ok(verifications) = CacheRepository::new().get_verification_by_project(project_id) {
        if let Ok(json_data) = serde_json::to_string(&verifications) {
            cache.set_with_ttl(
                &keys::Verification::by_project(project_id),
                json_data,
                Duration::from_secs(600)
            );
        }
    }
}

/// Cache warming for frequently accessed data
pub fn warm_frequently_accessed_cache() {
    let cache = get_cache();

    // Warm up matrix data for all projects
    if let Ok(projects) = CacheRepository::new().get_projects_all() {
        for project in projects {
            if let Ok(matrix_data) =
                CacheRepository::new().get_matrix_by_project(project.project_id)
            {
                if let Ok(json_data) = serde_json::to_string(&matrix_data) {
                    cache.set_with_ttl(
                        &keys::Matrix::by_project(project.project_id),
                        json_data,
                        Duration::from_secs(1800), // 30 minutes
                    );
                }
            }
        }
    }

    // Warm up user data with recent activity
    if let Ok(users) = CacheRepository::new().get_users_all() {
        for user in users {
            if let Ok(json_data) = serde_json::to_string(&user) {
                cache.set_with_ttl(
                    &keys::Users::by_id(user.user_id),
                    json_data,
                    Duration::from_secs(600),
                );
            }
        }
    }
}*/
