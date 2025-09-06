use crate::cache::{get_cache, keys, invalidate_project_cache, invalidate_user_cache, invalidate_requirement_cache, invalidate_test_cache, invalidate_category_cache};
use crate::repository::{
    DieselRepo, LookupRepository, MatrixRepository, ProjectsRepository, RequirementsRepository,
    TestsRepository, UserRepository,
};
use crate::repository::errors::RepoError;
use crate::helper_functions::decorators::get_linked_tests_for_requirement;
use crate::models::*;
use serde_json::{self, json};
use std::time::Duration;
use chrono;

/// Get projects for navigation with caching
pub fn get_projects_for_nav_cached() -> Result<Vec<Project>, String> {
    let cache = get_cache();
    
    // Try to get from cache first
    if let Some(cached_data) = cache.get(keys::PROJECTS_NAV) {
        match serde_json::from_str::<Vec<Project>>(&cached_data) {
            Ok(projects) => return Ok(projects),
            Err(_) => {
                // Invalid cache data, remove it
                cache.remove(keys::PROJECTS_NAV);
            }
        }
    }
    
    // Get from database and cache the result
    let repo = DieselRepo::new();
    let projects = repo.get_projects_all().map_err(|e| e.to_string())?;
    let json_data = serde_json::to_string(&projects)
        .map_err(|e| format!("Serialization error: {}", e))?;
    
    // Cache for 10 minutes (longer TTL for navigation data)
    cache.set_with_ttl(keys::PROJECTS_NAV, json_data, Duration::from_secs(600));
    
    Ok(projects)
}

/// Get all statuses with caching
pub fn get_status_all_cached() -> Result<Vec<Status>, String> {
    let cache = get_cache();
    
    if let Some(cached_data) = cache.get(keys::STATUS_ALL) {
        match serde_json::from_str::<Vec<Status>>(&cached_data) {
            Ok(statuses) => return Ok(statuses),
            Err(_) => {
                cache.remove(keys::STATUS_ALL);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let statuses = repo.get_status_all().map_err(|e| e.to_string())?;
    let json_data = serde_json::to_string(&statuses)
        .map_err(|e| format!("Serialization error: {}", e))?;
    
    // Cache for 15 minutes (status data rarely changes)
    cache.set_with_ttl(keys::STATUS_ALL, json_data, Duration::from_secs(900));
    
    Ok(statuses)
}

/// Get all categories with caching
pub fn get_categories_all_cached() -> Result<Vec<Category>, String> {
    let cache = get_cache();
    
    if let Some(cached_data) = cache.get(keys::CATEGORIES_ALL) {
        match serde_json::from_str::<Vec<Category>>(&cached_data) {
            Ok(categories) => return Ok(categories),
            Err(_) => {
                cache.remove(keys::CATEGORIES_ALL);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let categories = repo.get_categories_all().map_err(|e| e.to_string())?;
    let json_data = serde_json::to_string(&categories)
        .map_err(|e| format!("Serialization error: {}", e))?;
    
    // Cache for 10 minutes
    cache.set_with_ttl(keys::CATEGORIES_ALL, json_data, Duration::from_secs(600));
    
    Ok(categories)
}

/// Get all applicability with caching
pub fn get_applicability_all_cached() -> Result<Vec<Applicability>, String> {
    let cache = get_cache();
    
    if let Some(cached_data) = cache.get(keys::APPLICABILITY_ALL) {
        match serde_json::from_str::<Vec<Applicability>>(&cached_data) {
            Ok(applicability) => return Ok(applicability),
            Err(_) => {
                cache.remove(keys::APPLICABILITY_ALL);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let applicability = repo.get_applicability_all().map_err(|e| e.to_string())?;
    let json_data = serde_json::to_string(&applicability)
        .map_err(|e| format!("Serialization error: {}", e))?;
    
    // Cache for 10 minutes
    cache.set_with_ttl(keys::APPLICABILITY_ALL, json_data, Duration::from_secs(600));
    
    Ok(applicability)
}

/// Get all verification data with caching
pub fn get_verification_all_cached() -> Result<Vec<Verification>, String> {
    let cache = get_cache();
    
    if let Some(cached_data) = cache.get(keys::VERIFICATION_ALL) {
        match serde_json::from_str::<Vec<Verification>>(&cached_data) {
            Ok(verification) => return Ok(verification),
            Err(_) => {
                cache.remove(keys::VERIFICATION_ALL);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let verification = repo.get_verification_all().map_err(|e| e.to_string())?;
    let json_data = serde_json::to_string(&verification)
        .map_err(|e| format!("Serialization error: {}", e))?;
    
    // Cache for 10 minutes
    cache.set_with_ttl(keys::VERIFICATION_ALL, json_data, Duration::from_secs(600));
    
    Ok(verification)
}

/// Get all users with caching
pub fn get_users_all_cached() -> Result<Vec<User>, String> {
    let cache = get_cache();
    
    if let Some(cached_data) = cache.get(keys::USERS_ALL) {
        match serde_json::from_str::<Vec<User>>(&cached_data) {
            Ok(users) => return Ok(users),
            Err(_) => {
                cache.remove(keys::USERS_ALL);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let users = repo.get_users_all().map_err(|e| e.to_string())?;
    let json_data = serde_json::to_string(&users)
        .map_err(|e| format!("Serialization error: {}", e))?;
    
    // Cache for 5 minutes (user data might change more frequently)
    cache.set_with_ttl(keys::USERS_ALL, json_data, Duration::from_secs(300));
    
    Ok(users)
}

/// Get user by ID with caching
pub fn get_user_by_id_cached(id: i32) -> User {
    let cache = get_cache();
    let cache_key = keys::user_by_id(id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<User>(&cached_data) {
            Ok(user) => return user,
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let user = repo
        .get_user_by_id(id)
        .expect("Error reading table Users");
    let json_data = serde_json::to_string(&user)
        .unwrap_or_default();
    
    // Cache for 5 minutes
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(300));
    
    user
}

/// Get requirements by project with caching
pub fn get_requirements_by_project_cached(project_id: i32) -> Result<Vec<Requirement>, String> {
    let cache = get_cache();
    let cache_key = keys::requirements_by_project(project_id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Vec<Requirement>>(&cached_data) {
            Ok(requirements) => return Ok(requirements),
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let requirements = repo
        .get_requirements_by_project(project_id)
        .map_err(|e| e.to_string())?;
    let json_data = serde_json::to_string(&requirements)
        .map_err(|e| format!("Serialization error: {}", e))?;
    
    // Cache for 5 minutes
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(300));
    
    Ok(requirements)
}

/// Get tests by project with caching
pub fn get_tests_by_project_cached(project_id: i32) -> Result<Vec<Test>, String> {
    let cache = get_cache();
    let cache_key = keys::tests_by_project(project_id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Vec<Test>>(&cached_data) {
            Ok(tests) => return Ok(tests),
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let tests = repo
        .get_tests_by_project(project_id)
        .map_err(|e| e.to_string())?;
    let json_data = serde_json::to_string(&tests)
        .map_err(|e| format!("Serialization error: {}", e))?;
    
    // Cache for 5 minutes
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(300));
    
    Ok(tests)
}

/// Get matrix by project with caching
pub fn get_matrix_by_project_cached(project_id: i32) -> Result<Vec<Matrix>, String> {
    let cache = get_cache();
    let cache_key = keys::matrix_by_project(project_id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Vec<Matrix>>(&cached_data) {
            Ok(matrix) => return Ok(matrix),
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let matrix = repo
        .get_matrix_by_project(project_id)
        .map_err(|e| e.to_string())?;
    let json_data = serde_json::to_string(&matrix)
        .map_err(|e| format!("Serialization error: {}", e))?;
    
    // Cache for 3 minutes (matrix data is more dynamic)
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(180));
    
    Ok(matrix)
}

/// Get requirement by ID with caching
pub fn get_requirement_by_id_cached(id: i32) -> Requirement {
    let cache = get_cache();
    let cache_key = keys::requirement_by_id(id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Requirement>(&cached_data) {
            Ok(requirement) => return requirement,
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let requirement = repo.get_requirement_by_id(id).unwrap_or_else(|_| Requirement {
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
    });
    let json_data = serde_json::to_string(&requirement)
        .unwrap_or_default();
    
    // Cache for 5 minutes
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(300));
    
    requirement
}

/// Get requirement by ID with caching and proper error handling
pub fn get_requirement_by_id_cached_safe(id: i32) -> Result<Requirement, String> {
    let cache = get_cache();
    let cache_key = keys::requirement_by_id(id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Requirement>(&cached_data) {
            Ok(requirement) => return Ok(requirement),
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let requirement = repo
        .get_requirement_by_id(id)
        .map_err(|e| match e {
            RepoError::NotFound => format!("Requirement with ID {} not found", id),
            _ => e.to_string(),
        })?;
    let json_data = serde_json::to_string(&requirement)
        .unwrap_or_default();
    
    // Cache for 5 minutes
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(300));
    
    Ok(requirement)
}

/// Get test by ID with caching
pub fn get_test_by_id_cached(id: i32) -> Test {
    let cache = get_cache();
    let cache_key = keys::test_by_id(id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Test>(&cached_data) {
            Ok(test) => return test,
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let test = repo
        .get_test_by_id(id)
        .expect("Error reading table Tests");
    let json_data = serde_json::to_string(&test)
        .unwrap_or_default();
    
    // Cache for 5 minutes
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(300));
    
    test
}

/// Get test by ID with caching and proper error handling
pub fn get_test_by_id_cached_safe(id: i32) -> Result<Test, String> {
    let cache = get_cache();
    let cache_key = keys::test_by_id(id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Test>(&cached_data) {
            Ok(test) => return Ok(test),
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let test = repo
        .get_test_by_id(id)
        .map_err(|e| match e {
            RepoError::NotFound => format!("Test with ID {} not found", id),
            _ => e.to_string(),
        })?;
    let json_data = serde_json::to_string(&test)
        .unwrap_or_default();
    
    // Cache for 5 minutes
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(300));
    
    Ok(test)
}

/// Get category by ID with caching
pub fn get_category_by_id_cached(id: i32) -> Category {
    let cache = get_cache();
    let cache_key = keys::category_by_id(id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Category>(&cached_data) {
            Ok(category) => return category,
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let category = repo.get_category_by_id(id).unwrap_or_else(|_| Category {
        cat_id: id,
        cat_title: format!("Unknown Category ({})", id),
        cat_description: "Category not found".to_string(),
        cat_tag: "unknown".to_string(),
        project_id: 1,
    });
    let json_data = serde_json::to_string(&category)
        .unwrap_or_default();
    
    // Cache for 10 minutes
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(600));
    
    category
}

/// Cached version of get_requirements_all with project filtering
pub fn get_requirements_all_cached() -> Result<Vec<Requirement>, String> {
    DieselRepo::new()
        .get_requirements_all()
        .map_err(|e| e.to_string())
}

/// Cached version of get_tests_all with project filtering
pub fn get_tests_all_cached() -> Result<Vec<Test>, String> {
    DieselRepo::new()
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
    let cache = get_cache();
    let cache_key = format!("verification:project:{}", project_id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Vec<Verification>>(&cached_data) {
            Ok(verification) => return Ok(verification),
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let verification = repo
        .get_verification_by_project(project_id)
        .map_err(|e| e.to_string())?;
    let json_data = serde_json::to_string(&verification)
        .map_err(|e| format!("Serialization error: {}", e))?;
    
    // Cache for 5 minutes
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(300));
    
    Ok(verification)
}

/// Get categories by project with caching
pub fn get_categories_by_project_cached(project_id: i32) -> Result<Vec<Category>, String> {
    let cache = get_cache();
    let cache_key = format!("categories:project:{}", project_id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Vec<Category>>(&cached_data) {
            Ok(categories) => return Ok(categories),
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let categories = repo
        .get_categories_by_project(project_id)
        .map_err(|e| e.to_string())?;
    let json_data = serde_json::to_string(&categories)
        .map_err(|e| format!("Serialization error: {}", e))?;
    
    // Cache for 5 minutes
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(300));
    
    Ok(categories)
}

/// Get applicability by project with caching
pub fn get_applicability_by_project_cached(project_id: i32) -> Result<Vec<Applicability>, String> {
    let cache = get_cache();
    let cache_key = format!("applicability:project:{}", project_id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Vec<Applicability>>(&cached_data) {
            Ok(applicability) => return Ok(applicability),
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let applicability = repo
        .get_applicability_by_project(project_id)
        .map_err(|e| e.to_string())?;
    let json_data = serde_json::to_string(&applicability)
        .map_err(|e| format!("Serialization error: {}", e))?;
    
    // Cache for 5 minutes
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(300));
    
    Ok(applicability)
}

/// Get linked tests for requirement with caching
pub fn get_linked_tests_for_requirement_cached(req_id: i32) -> Result<Vec<DecoratedTest>, String> {
    let cache = get_cache();
    let cache_key = format!("linked_tests:requirement:{}", req_id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Vec<DecoratedTest>>(&cached_data) {
            Ok(tests) => return Ok(tests),
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }

    let tests = get_linked_tests_for_requirement(req_id)
        .map_err(|e| format!("Database error: {}", e))?;

    let json_data = serde_json::to_string(&tests)
        .map_err(|e| format!("Serialization error: {}", e))?;

    // Cache for 3 minutes (linked tests can change frequently)
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(180));

    Ok(tests)
}

/// Get requirements for test with caching
pub fn get_requirements_for_test_cached(test_id: i32) -> Result<Vec<Requirement>, String> {
    let cache = get_cache();
    let cache_key = format!("requirements:test:{}", test_id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Vec<Requirement>>(&cached_data) {
            Ok(requirements) => return Ok(requirements),
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let requirements = repo
        .get_requirements_for_test(test_id)
        .map_err(|e| e.to_string())?;
    let json_data = serde_json::to_string(&requirements)
        .map_err(|e| format!("Serialization error: {}", e))?;
    
    // Cache for 5 minutes
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(300));
    Ok(requirements)
}

/// Get status by ID with caching
pub fn get_status_by_id_cached(id: i32) -> Status {
    let cache = get_cache();
    let cache_key = format!("status:{}", id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Status>(&cached_data) {
            Ok(status) => return status,
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let status = repo
        .get_status_by_id(id)
        .expect("Error reading table Status");
    let json_data = serde_json::to_string(&status)
        .unwrap_or_default();
    
    // Cache for 15 minutes (status data rarely changes)
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(900));
    
    status
}

/// Get verification by ID with caching
pub fn get_verification_by_id_cached(id: i32) -> Verification {
    let cache = get_cache();
    let cache_key = format!("verification:{}", id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Verification>(&cached_data) {
            Ok(verification) => return verification,
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let verification = repo
        .get_verification_by_id(id)
        .unwrap_or_else(|_| Verification {
            verification_id: id,
            verification_name: format!("Unknown Verification ({})", id),
            verification_description: "Verification not found".to_string(),
            project_id: 1,
        });
    let json_data = serde_json::to_string(&verification)
        .unwrap_or_default();
    
    // Cache for 10 minutes
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(600));
    
    verification
}

/// Get applicability by ID with caching
pub fn get_applicability_by_id_cached(id: i32) -> Applicability {
    let cache = get_cache();
    let cache_key = format!("applicability:{}", id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Applicability>(&cached_data) {
            Ok(applicability) => return applicability,
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let applicability = repo
        .get_applicability_by_id(id)
        .unwrap_or_else(|_| Applicability {
            app_id: id,
            app_title: format!("Unknown Applicability ({})", id),
            app_description: "Applicability not found".to_string(),
            app_tag: "unknown".to_string(),
            project_id: 1,
        });
    let json_data = serde_json::to_string(&applicability)
        .unwrap_or_default();
    
    // Cache for 10 minutes
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(600));
    
    applicability
}

/// Get project by ID with caching
pub fn get_project_by_id_cached(project_id: i32) -> Project {
    let cache = get_cache();
    let cache_key = format!("project:{}", project_id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        match serde_json::from_str::<Project>(&cached_data) {
            Ok(project) => return project,
            Err(_) => {
                cache.remove(&cache_key);
            }
        }
    }
    
    let repo = DieselRepo::new();
    let project = repo
        .get_project_by_id(project_id)
        .expect("Error loading project");
    let json_data = serde_json::to_string(&project)
        .unwrap_or_default();
    
    // Cache for 15 minutes (project data rarely changes)
    cache.set_with_ttl(&cache_key, json_data, Duration::from_secs(900));
    
    project
}

/// Get requirement title by ID with caching
pub fn get_requirement_title_by_id_cached(id: i32) -> String {
    let cache = get_cache();
    let cache_key = format!("requirement_title:{}", id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        return cached_data;
    }
    
    let repo = DieselRepo::new();
    let title = repo
        .get_requirement_by_id(id)
        .map(|r| r.req_title)
        .unwrap_or_else(|_| "[Requirement Not Found]".to_string());
    
    // Cache for 10 minutes
    cache.set_with_ttl(&cache_key, title.clone(), Duration::from_secs(600));
    
    title
}

/// Get test status by ID with caching
pub fn get_test_status_by_id_cached(id: i32) -> String {
    let cache = get_cache();
    let cache_key = format!("test_status:{}", id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        return cached_data;
    }
    
    let repo = DieselRepo::new();
    let status = {
        if let Ok(test) = repo.get_test_by_id(id) {
            repo.get_status_by_id(test.test_status)
                .map(|s| s.st_title)
                .unwrap_or_else(|_| "[Status Not Found]".to_string())
        } else {
            "[Test Not Found]".to_string()
        }
    };
    
    // Cache for 5 minutes
    cache.set_with_ttl(&cache_key, status.clone(), Duration::from_secs(300));
    
    status
}

/// Get status name by ID with caching
pub fn get_status_name_by_id_cached(id: i32) -> String {
    let cache = get_cache();
    let cache_key = format!("status_name:{}", id);
    
    if let Some(cached_data) = cache.get(&cache_key) {
        return cached_data;
    }
    
    let repo = DieselRepo::new();
    let name = repo
        .get_status_by_id(id)
        .map(|s| s.st_title)
        .unwrap_or_else(|_| "[Status Not Found]".to_string());
    
    // Cache for 15 minutes (status names rarely change)
    cache.set_with_ttl(&cache_key, name.clone(), Duration::from_secs(900));
    
    name
}

/// Warm up frequently accessed cache entries
/* TODO: never used ??
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
}*/

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
            if let Ok(projects) = DieselRepo::new().get_projects_all() {
                for project in projects {
                    cache.remove(&keys::requirements_by_project(project.project_id));
                }
            }
        }
        "test" => {
            for &id in entity_ids {
                invalidate_test_cache(id);
            }
            // Also invalidate project-specific caches
            if let Ok(projects) = DieselRepo::new().get_projects_all() {
                for project in projects {
                    cache.remove(&keys::tests_by_project(project.project_id));
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
            if let Ok(projects) = DieselRepo::new().get_projects_all() {
                for project in projects {
                    cache.remove(&keys::requirements_by_project(project.project_id));
                    cache.remove(&keys::tests_by_project(project.project_id));
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
/* TODO: never used ??
pub fn warm_project_cache(project_id: i32) {
    let cache = get_cache();
    
    // Warm up project-specific requirements
    if let Ok(requirements) = DieselRepo::new().get_requirements_by_project(project_id) {
        if let Ok(json_data) = serde_json::to_string(&requirements) {
            cache.set_with_ttl(
                &keys::requirements_by_project(project_id),
                json_data,
                Duration::from_secs(300)
            );
        }
    }
    
    // Warm up project-specific tests
    if let Ok(tests) = DieselRepo::new().get_tests_by_project(project_id) {
        if let Ok(json_data) = serde_json::to_string(&tests) {
            cache.set_with_ttl(
                &keys::tests_by_project(project_id),
                json_data,
                Duration::from_secs(300)
            );
        }
    }
    
    // Warm up project-specific categories
    if let Ok(categories) = DieselRepo::new().get_categories_by_project(project_id) {
        if let Ok(json_data) = serde_json::to_string(&categories) {
            cache.set_with_ttl(
                &keys::categories_by_project(project_id),
                json_data,
                Duration::from_secs(600)
            );
        }
    }
    
    // Warm up project-specific verification types
    if let Ok(verifications) = DieselRepo::new().get_verification_by_project(project_id) {
        if let Ok(json_data) = serde_json::to_string(&verifications) {
            cache.set_with_ttl(
                &keys::verification_by_project(project_id),
                json_data,
                Duration::from_secs(600)
            );
        }
    }
}*/

/// Cache warming for frequently accessed data
pub fn warm_frequently_accessed_cache() {
    let cache = get_cache();
    
    // Warm up matrix data for all projects
    if let Ok(projects) = DieselRepo::new().get_projects_all() {
        for project in projects {
            if let Ok(matrix_data) = DieselRepo::new().get_matrix_by_project(project.project_id) {
                if let Ok(json_data) = serde_json::to_string(&matrix_data) {
                    cache.set_with_ttl(
                        &keys::matrix_by_project(project.project_id),
                        json_data,
                        Duration::from_secs(1800) // 30 minutes
                    );
                }
            }
        }
    }
    
    // Warm up user data with recent activity
    if let Ok(users) = DieselRepo::new().get_users_all() {
        for user in users {
            if let Ok(json_data) = serde_json::to_string(&user) {
                cache.set_with_ttl(
                    &keys::user_by_id(user.user_id),
                    json_data,
                    Duration::from_secs(600)
                );
            }
        }
    }
}

/// Get all projects with caching
pub fn get_projects_all_cached() -> Result<Vec<Project>, Box<dyn std::error::Error>> {
    let cache = get_cache();
    let cache_key = keys::PROJECTS_ALL;
    
    if let Some(cached_data) = cache.get(cache_key) {
        match serde_json::from_str::<Vec<Project>>(&cached_data) {
            Ok(projects) => return Ok(projects),
            Err(_) => {
                cache.remove(cache_key);
            }
        }
    }
    
    let projects = DieselRepo::new().get_projects_all()?;
    let json_data = serde_json::to_string(&projects)?;
    
    // Cache for 10 minutes (project data changes occasionally)
    cache.set_with_ttl(cache_key, json_data, Duration::from_secs(600));
    
    Ok(projects)
}
