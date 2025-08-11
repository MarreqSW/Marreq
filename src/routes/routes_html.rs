use diesel::prelude::*;
use rocket::form::Form;
use rocket::fs::NamedFile;
use rocket::http::ContentType;
use rocket::response::status::NotFound;
use rocket::response::{content, Redirect};
use rocket::serde::json::json;
use rocket::http::{Cookie, CookieJar};
use regex::Regex;
use chrono;

use rocket_dyn_templates::Template;

use std::path;

use crate::generators::*;
use crate::helper_functions::*;
use crate::cached_functions::*;
use crate::html::*;
use crate::logger::Logger;
use crate::models::*;
use crate::db_operations::*;
use crate::db::{get_pooled_connection, get_connection_pooled_safe, PooledConnectionWrapper};

// --------------------------------
// Helper Functions
// --------------------------------

/// Helper function to get a database connection with proper error handling
fn get_db_connection() -> Result<PooledConnectionWrapper, Box<dyn std::error::Error>> {
    get_connection_pooled_safe()
}

// --------------------------------
// Authentication Helper Functions
// --------------------------------

pub fn require_auth(cookies: &CookieJar<'_>) -> Result<User, Redirect> {
    match is_authenticated(cookies) {
        Some(user) => Ok(user),
        None => Err(Redirect::to(uri!(login_page)))
    }
}

fn build_context_with_projects(user: User, cookies: &CookieJar<'_>) -> rocket::serde::json::Value {
    let projects = get_projects_for_nav_cached().unwrap_or_default();
    let selected_project_id = get_selected_project_id(cookies);
    
    json!({
        "user": user,
        "projects": projects,
        "selected_project_id": selected_project_id
    })
}

// --------------------------------
// Authentication Routes
// --------------------------------

#[get("/login")]
pub fn login_page() -> Template {
    // Get projects for navigation (even on login page)
    let projects = get_projects_for_nav_cached().unwrap_or_default();
    let selected_project_id: Option<i32> = None; // No project selected on login page
    
    let ctx = json!({
        "title": "Login",
        "projects": projects,
        "selected_project_id": selected_project_id
    });
    Template::render("login", ctx)
}

#[post("/login", data = "<login_form>")]
pub fn login(login_form: Form<LoginForm>, cookies: &CookieJar<'_>) -> Result<Redirect, Template> {
    match authenticate_user(&login_form.username, &login_form.password) {
        Ok(Some(user)) => {
            // Set session cookie
            cookies.add_private(Cookie::new("user_id", user.user_id.to_string()));
            cookies.add_private(Cookie::new("username", user.user_username.clone()));
            cookies.add_private(Cookie::new("user_name", user.user_name.clone()));
            
            // Log successful login
            let mut conn = get_db_connection().map_err(|e| {
                eprintln!("Database connection error: {}", e);
                Template::render("error", json!({"error": "Database connection failed"}))
            })?;
            let _ = Logger::log_login(
                &mut conn,
                user.user_id,
                None,
            );
            
            Ok(Redirect::to(uri!(index)))
        }
        Ok(None) => {
            let ctx = json!({
                "title": "Login",
                "error": "Invalid username or password"
            });
            Err(Template::render("login", ctx))
        }
        Err(_e) => {
            let ctx = json!({
                "title": "Login",
                "error": format!("Authentication error: {}", _e)
            });
            Err(Template::render("login", ctx))
        }
    }
}

#[get("/logout")]
pub fn logout(cookies: &CookieJar<'_>) -> Redirect {
    // Get user info from cookies before clearing them
    let user_id = cookies.get_private("user_id")
        .and_then(|cookie| cookie.value().parse::<i32>().ok());
    let username = cookies.get_private("username")
        .map(|cookie| cookie.value().to_string());
    
    // Clear all session cookies
    let mut user_id_cookie = Cookie::new("user_id", "");
    user_id_cookie.set_max_age(time::Duration::seconds(0));
    user_id_cookie.set_path("/");
    
    let mut username_cookie = Cookie::new("username", "");
    username_cookie.set_max_age(time::Duration::seconds(0));
    username_cookie.set_path("/");
    
    let mut user_name_cookie = Cookie::new("user_name", "");
    user_name_cookie.set_max_age(time::Duration::seconds(0));
    user_name_cookie.set_path("/");
    
    // Add the expired cookies to force removal
    cookies.add_private(user_id_cookie);
    cookies.add_private(username_cookie);
    cookies.add_private(user_name_cookie);
    
    // Log logout if we have user info
    if let Some(uid) = user_id {
        if let Ok(mut conn) = get_db_connection() {
            let _description = username.map(|name| format!("User {} logged out", name));
            let _ = Logger::log_logout(
                &mut conn,
                uid,
                None,
            );
        }
    }
    
    Redirect::to(uri!(login_page))
}

#[get("/change_password")]
pub fn change_password_page() -> Template {
    // Get projects for navigation
    let projects = get_projects_for_nav().unwrap_or_default();
    let selected_project_id: Option<i32> = None; // No project selected on change password page
    
    let ctx = json!({
        "title": "Change Password",
        "projects": projects,
        "selected_project_id": selected_project_id
    });
    Template::render("change_password", ctx)
}

#[post("/change_password", data = "<password_form>")]
pub fn change_password(password_form: Form<ChangePasswordForm>, cookies: &CookieJar<'_>) -> Result<Template, Template> {
    // Get user ID from cookie
    let user_id_cookie = cookies.get_private("user_id");
    let user_id = match user_id_cookie {
        Some(cookie) => match cookie.value().parse::<i32>() {
            Ok(id) => id,
            Err(_) => {
                let ctx = json!({
                    "title": "Change Password",
                    "error": "Invalid session"
                });
                return Err(Template::render("change_password", ctx));
            }
        },
        None => {
            let ctx = json!({
                "title": "Change Password",
                "error": "Not logged in"
            });
            return Err(Template::render("change_password", ctx));
        }
    };
    
    // Validate passwords
    if password_form.new_password != password_form.confirm_password {
        let ctx = json!({
            "title": "Change Password",
            "error": "New passwords do not match"
        });
        return Err(Template::render("change_password", ctx));
    }
    
    if password_form.new_password.len() < 8 {
        let ctx = json!({
            "title": "Change Password",
            "error": "New password must be at least 8 characters long"
        });
        return Err(Template::render("change_password", ctx));
    }
    
    // Change password
    match change_user_password(user_id, &password_form.current_password, &password_form.new_password) {
        Ok(_) => {
            let ctx = json!({
                "title": "Change Password",
                "success": "Password changed successfully"
            });
            Ok(Template::render("change_password", ctx))
        }
        Err(_e) => {
            let ctx = json!({
                "title": "Change Password",
                "error": _e
            });
            Err(Template::render("change_password", ctx))
        }
    }
}

// --------------------------------
// Html Routes (TBD)
// --------------------------------

#[get("/")]
pub fn index(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Get selected project ID
    let selected_project_id = get_selected_project_id(cookies);
    
    // Get selected project name
    let selected_project_name = if let Some(project_id) = selected_project_id {
        let project = get_project_by_id_pooled_safe(project_id);
        project.project_name
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all_cached().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            first_project.project_name.clone()
        } else {
            "Requirements Manager".to_string()
        }
    };
    
    // Get counts for requirements and tests
    let requirements_count = if let Some(project_id) = selected_project_id {
        get_requirements_by_project(project_id).map(|reqs| reqs.len()).unwrap_or(0)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all_cached().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_requirements_by_project(first_project.project_id).map(|reqs| reqs.len()).unwrap_or(0)
        } else {
            get_requirements_all_cached().map(|reqs| reqs.len()).unwrap_or(0)
        }
    };
    
    let tests_count = if let Some(project_id) = selected_project_id {
        get_tests_by_project(project_id).map(|tests| tests.len()).unwrap_or(0)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all_cached().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_tests_by_project(first_project.project_id).map(|tests| tests.len()).unwrap_or(0)
        } else {
            get_tests_all_cached().map(|tests| tests.len()).unwrap_or(0)
        }
    };
    
    let projects = get_projects_for_nav().unwrap_or_default();
    
    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": selected_project_id,
        "title": "Main",
        "selected_project_name": selected_project_name,
        "requirements_count": requirements_count,
        "tests_count": tests_count
    });
    
    Ok(Template::render("index", ctx))
}

#[get("/requirements?<status_filter>&<verification_filter>&<category_filter>")]
pub fn show_requirements(
    cookies: &CookieJar<'_>,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let mut ctx = build_context_with_projects(user, cookies);
    
    // Get selected project ID
    let selected_project_id = get_selected_project_id(cookies);
    
    let requirements = if let Some(project_id) = selected_project_id {
        get_requirements_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_requirements_by_project_cached(first_project.project_id)
        } else {
            get_requirements_all_cached()
        }
    };

    match requirements {
        Ok(req) => {
            // Apply filters
            let filtered_requirements = filter_requirements(req, status_filter, verification_filter, category_filter);
            let requirements_decorate = decorate_requirements(filtered_requirements);
            ctx["requirements"] = json!(requirements_decorate);
        }
        Err(_) => {
            ctx["requirements"] = json!([]);
        }
    };

    // Add filter data to context for the template
    let statuses = get_status_all_cached().unwrap_or_default();
    let verifications = if let Some(project_id) = selected_project_id {
        get_verification_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_verification_by_project_cached(first_project.project_id)
        } else {
            get_verification_all_cached()
        }
    };
    
    // Get categories filtered by selected project
    let categories = if let Some(project_id) = selected_project_id {
        get_categories_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_categories_by_project_cached(first_project.project_id)
        } else {
            get_categories_all_cached()
        }
    };
    
    ctx["statuses"] = json!(statuses);
    ctx["verifications"] = json!(verifications.unwrap_or_default());
    ctx["categories"] = json!(categories.unwrap_or_default());
    ctx["current_status_filter"] = json!(status_filter);
    ctx["current_verification_filter"] = json!(verification_filter);
    ctx["current_category_filter"] = json!(category_filter);

    Ok(Template::render("requirements", ctx))
}

#[get("/requirements/<req_id>")]
pub fn show_requirement_id(req_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Use the safe function that returns a Result
    match get_requirement_by_id_cached_safe(req_id) {
        Ok(req) => {
            let req_decorate = decorate_requirements(vec![req]);

            // Get linked tests for this requirement
            let linked_tests = get_linked_tests_for_requirement_cached(req_id).unwrap_or_default();
            let linked_tests_json = json!(linked_tests);

            let ctx = json!({
                "requirements": req_decorate,
                "linked_tests": linked_tests_json,
                "user": user
            });

            Ok(Template::render("requirement_by_id", ctx))
        },
        Err(error_msg) => {
            // Render error template instead of panicking
            let ctx = json!({
                "title": "Requirement Not Found",
                "message": "The requirement you're looking for could not be found.",
                "details": error_msg,
                "user": user
            });
            
            Ok(Template::render("error", ctx))
        }
    }
}

#[get("/users")]
pub fn show_users(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let users = get_users_all_cached();

    let ctx = match users {
        Ok(users_list) => {
            json!({
                "users": users_list,
                "user": user
            })
        }
        Err(_) => {
            json!({
                "users": [],
                "user": user
            })
        }
    };

    Ok(Template::render("users", ctx))
}

#[get("/users/<user_id>")]
pub fn show_user_id(user_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let current_user = require_auth(cookies)?;
    let user = get_user_by_id_cached(user_id);
    let ctx = json!({
        "user": current_user,
        "user_name": user.user_name,
        "user_username": user.user_username,
        "user_email": user.user_email,
        "user_level": user.user_level,
        "user_id": user.user_id,
        "user_creation_date": user.user_creation_date,
        "user_last_login": user.user_last_login
    });

    Ok(Template::render("user_by_id", ctx))
}

#[get("/edit_user/<user_id>")]
pub fn edit_user(user_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let current_user = require_auth(cookies)?;
    let user = get_user_by_id(user_id);
    #[cfg(debug_assertions)]
    println!("USer: {:?}", user);
    let ctx = json!({
        "users": user,
        "user": current_user
    });
    #[cfg(debug_assertions)]
    println!("edit user: {:?}", ctx);
    Ok(Template::render("edit_user_by_id", ctx))
}

#[post("/edit_user/<user_id>", data = "<user_form>")]
pub fn post_edit_user(user_id: i32, user_form: Form<UpdateUser>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let current_user = require_auth(cookies)?;
    
    let connection = &mut get_db_connection().map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(edit_user(user_id)))
    })?;
    
    // Get the old values before updating
    let old_user = get_user_by_id(user_id);
    
    // Create an UpdateUser with the user_id
    let mut user_data = user_form.into_inner();
    user_data.user_id = Some(user_id);
    
    // Update the user in the database
    match update_user_without_password(connection, &user_data) {
        Ok(_) => {
            // Log the user update
            if let (Ok(old_values), Ok(new_values)) = (Logger::to_json_string(&old_user), Logger::to_json_string(&user_data)) {
                let _ = Logger::log_update(
                    connection,
                    current_user.user_id,
                    EntityType::User,
                    user_id,
                    None,
                    Some(old_values),
                    Some(new_values),
                    Some(format!("Updated user: {}", user_data.user_username)),
                    None,
                );
            }
            
            // Invalidate cache for the updated user
            invalidate_user_cache_complete(user_id);
            
            Ok(Redirect::to(uri!(show_user_id(user_id))))
        },
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(edit_user(user_id))))
        }
    }
}

#[get("/edit_requirement/<req_id>")]
pub fn get_edit_requirement(req_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Use the safe function that returns a Result
    let req = match get_requirement_by_id_safe(req_id) {
        Ok(req) => req,
        Err(error_msg) => {
            // Render error template instead of panicking
            let ctx = json!({
                "title": "Requirement Not Found",
                "message": "The requirement you're trying to edit could not be found.",
                "details": error_msg,
                "user": user
            });
            
            return Ok(Template::render("error", ctx));
        }
    };
    
    let req_decorate = decorate_requirements(vec![req.clone()]);
    let req_decorate_json = json!(req_decorate[0]);

    let status = get_status_all_cached().unwrap_or_default();
    let status_json = json!(status);

    // Get selected project ID and filter categories accordingly
    let selected_project_id = get_selected_project_id(cookies);
    let categories = if let Some(project_id) = selected_project_id {
        get_categories_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_categories_by_project_cached(first_project.project_id)
        } else {
            get_categories_all_cached()
        }
    };
    let categories_json = json!(categories.unwrap_or_default());

    // Get parent requirements filtered by project
    let parents = if let Some(project_id) = selected_project_id {
        get_requirements_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_requirements_by_project(first_project.project_id)
        } else {
            get_requirements_all_cached()
        }
    };
    let parents_json = json!(parents.unwrap_or_default());

    let users = get_users_all().unwrap_or_default();
    let users_json = json!(users);

    // Get verification types filtered by project
    let verification_types = if let Some(project_id) = selected_project_id {
        get_verification_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_verification_by_project_cached(first_project.project_id)
        } else {
            get_verification_all_cached()
        }
    };
    let verification_json = json!(verification_types.unwrap_or_default());

    // Get applicability filtered by project
    let applicability = if let Some(project_id) = selected_project_id {
        get_applicability_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_applicability_by_project_cached(first_project.project_id)
        } else {
            get_applicability_all_cached()
        }
    };
    let applicability_json = json!(applicability.unwrap_or_default());

    let ctx = json!({
        "requirements": req_decorate_json, 
        "req_author_id": req.req_author,
        "req_reviewer_id": req.req_reviewer,
        "req_category_id": req.req_category,
        "req_applicability_id": req.req_applicability,
        "req_current_status_id": req.req_current_status,
        "req_verification_id": req.req_verification,
        "req_parent_id": req.req_parent,
        "categories": categories_json, 
        "status": status_json, 
        "parent": parents_json, 
        "users": users_json, 
        "verification": verification_json, 
        "applicability": applicability_json,
        "user": user
    });

    #[cfg(debug_assertions)]
    println!("Requirement: {:#}", ctx);
    Ok(Template::render("edit_requirement_by_id", ctx))
}

#[post("/edit_requirement/<req_id>", data = "<new_req>")]
pub fn post_edit_requirement(req_id: i32, new_req: Form<NewRequirement>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let user = require_auth(cookies)?;
    let my_id = new_req.req_id.unwrap_or(0);

    let requirement_data = new_req.into_inner();
    
    // Server-side validation: Check if reference follows general format
    if !requirement_data.req_reference.is_empty() {
        // Check general format: REQ-TAG-NUMBER
        let general_pattern = match Regex::new(r"^REQ-[A-Z]+-\d+$") {
            Ok(pattern) => pattern,
            Err(_) => {
                eprintln!("Failed to compile regex pattern");
                return Err(Redirect::to(uri!(get_edit_requirement(req_id))));
            }
        };
        if !general_pattern.is_match(&requirement_data.req_reference) {
            // Invalid reference format - redirect back to form with error
            return Err(Redirect::to(uri!(get_edit_requirement(req_id))));
        }
        
        // Get the category to check if reference matches
        let category = get_category_by_id(requirement_data.req_category);
        let expected_prefix = format!("REQ-{}-", category.cat_tag);
        
        // Only warn if reference doesn't match category, but don't block the update
        if !requirement_data.req_reference.starts_with(&expected_prefix) {
            // Log a warning but continue with the update
            println!("Warning: Reference '{}' doesn't match category tag '{}' for requirement {}", 
                     requirement_data.req_reference, category.cat_tag, req_id);
        }
    }

    let connection = &mut get_db_connection().map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(post_edit_requirement(req_id)))
    })?;
    
    // Get the old values before updating
    let old_requirement = match get_requirement_by_id_safe(req_id) {
        Ok(req) => req,
        Err(_) => {
            // Requirement not found - redirect back to requirements list
            return Err(Redirect::to(uri!(show_requirements(None::<i32>, None::<i32>, None::<i32>))));
        }
    };
    
    edit_requirement(connection, &requirement_data)
        .map_err(|e| {
            eprintln!("Error editing requirement: {:?}", e);
            Redirect::to(uri!(show_requirements(None::<i32>, None::<i32>, None::<i32>)))
        })?;

    // Log the requirement update
    if let (Ok(old_values), Ok(new_values)) = (
                    Logger::to_json_string(&old_requirement),
            Logger::to_json_string(&requirement_data)
    ) {
        let _ = Logger::log_update(
            connection,
            user.user_id,
            EntityType::Requirement,
            req_id,
            Some(requirement_data.project_id),
            Some(old_values),
            Some(new_values),
            Some(format!("Updated requirement: {}", requirement_data.req_title)),
            None,
        );
    }

    // Invalidate cache for the updated requirement
    invalidate_requirement_cache_complete(req_id);

    Ok(Redirect::to(uri!(show_requirement_id(my_id))))
}

#[delete("/delete_requirement/<req_id>")]
pub fn delete_requirement_route(req_id: i32, cookies: &CookieJar<'_>) -> Result<Redirect, rocket::http::Status> {
    let user = require_auth(cookies).map_err(|_| rocket::http::Status::Unauthorized)?;
    let mut connection = match get_db_connection() {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(rocket::http::Status::InternalServerError);
        }
    };
    
    // Get the requirement details before deleting
    let requirement = match get_requirement_by_id_safe(req_id) {
        Ok(req) => req,
        Err(_) => {
            // Requirement not found
            return Err(rocket::http::Status::NotFound);
        }
    };
    
    // Check if user can delete this requirement
    // Only allow deletion if status is Draft (1) or Proposal (2), or if user is admin
    if requirement.req_current_status > 2 && !user.is_admin {
        return Err(rocket::http::Status::Forbidden);
    }
    
    let result = delete_requirement(connection.as_mut(), &req_id);
    match result {
        Ok(success) => {
            if success {
                // Log the requirement deletion
                if let Ok(old_values) = Logger::to_json_string(&requirement) {
                    let _ = Logger::log_delete(
                        connection.as_mut(),
                        user.user_id,
                        EntityType::Requirement,
                        req_id,
                        Some(requirement.project_id),
                        Some(old_values),
                        Some(format!("Deleted requirement: {}", requirement.req_title)),
                        None,
                    );
                }
                
                // Invalidate related caches - including project-level caches
                crate::cached_functions::invalidate_requirement_cache_complete(req_id);
                
                // Also invalidate project-specific caches for the requirement's project
                crate::cached_functions::invalidate_project_cache_complete(requirement.project_id);
                
                // Invalidate the requirements list cache
                crate::cache::get_cache().remove(crate::cache::keys::REQUIREMENTS_ALL);
                
                // Redirect to requirements list page
                Ok(Redirect::to(uri!(show_requirements(None::<i32>, None::<i32>, None::<i32>))))
            } else {
                // Requirement was not found or not deleted
                Err(rocket::http::Status::NotFound)
            }
        },
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error deleting requirement: {:?}", _e);
            Err(rocket::http::Status::InternalServerError)
        }
    }
}

#[delete("/delete_test/<test_id>")]
pub fn delete_test_route(test_id: i32, cookies: &CookieJar<'_>) -> Result<Redirect, rocket::http::Status> {
    let user = require_auth(cookies).map_err(|_| rocket::http::Status::Unauthorized)?;
    let connection = &mut get_db_connection().map_err(|e| {
        eprintln!("Database connection error: {}", e);
        rocket::http::Status::InternalServerError
    })?;
    
    // Get the test details before deleting
    let test = match get_test_by_id_safe(test_id) {
        Ok(t) => t,
        Err(_) => {
            // Test not found
            return Err(rocket::http::Status::NotFound);
        }
    };
    
    // Check if user can delete this test
    // Only allow deletion if status is Draft (1) or Proposal (2), or if user is admin
    if test.test_status > 2 && !user.is_admin {
        return Err(rocket::http::Status::Forbidden);
    }
    
    let result = delete_test(connection, &test_id);
    match result {
        Ok(success) => {
            if success {
                // Log the test deletion
                if let Ok(old_values) = Logger::to_json_string(&test) {
                    let _ = Logger::log_delete(
                        connection,
                        user.user_id,
                        EntityType::Test,
                        test_id,
                        Some(test.project_id),
                        Some(old_values),
                        Some(format!("Deleted test: {}", test.test_name)),
                        None,
                    );
                }
                
                // Invalidate related caches - including project-level caches
                crate::cached_functions::invalidate_test_cache_complete(test_id);
                
                // Also invalidate project-specific caches for the test's project
                crate::cached_functions::invalidate_project_cache_complete(test.project_id);
                
                // Invalidate the tests list cache
                crate::cache::get_cache().remove(crate::cache::keys::TESTS_ALL);
                
                // Redirect to tests list page
                Ok(Redirect::to(uri!(show_tests(None::<i32>, None::<i32>, None::<i32>))))
            } else {
                // Test was not found or not deleted
                Err(rocket::http::Status::NotFound)
            }
        },
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error deleting test: {:?}", _e);
            Err(rocket::http::Status::InternalServerError)
        }
    }
}

#[get("/new_requirement")]
pub fn new_requirement(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let status = get_status_all_cached().unwrap_or_default();
    let status_json = json!(status);

    // Get selected project ID and filter categories accordingly
    let selected_project_id = get_selected_project_id(cookies);
    let categories = if let Some(project_id) = selected_project_id {
        get_categories_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_categories_by_project_cached(first_project.project_id)
        } else {
            get_categories_all_cached()
        }
    };
    let categories_json = json!(categories.unwrap_or_default());

    // Get parent requirements filtered by project
    let parents = if let Some(project_id) = selected_project_id {
        get_requirements_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all_cached().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_requirements_by_project_cached(first_project.project_id)
        } else {
            get_requirements_all_cached()
        }
    };
    let parents_json = json!(parents.unwrap_or_default());

    let users = get_users_all_cached().unwrap_or_default();
    let users_json = json!(users);

    // Get verification types filtered by project
    let verification_types = if let Some(project_id) = selected_project_id {
        get_verification_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_verification_by_project_cached(first_project.project_id)
        } else {
            get_verification_all_cached()
        }
    };
    let verification_json = json!(verification_types.unwrap_or_default());

    // Get applicability filtered by project
    let applicability = if let Some(project_id) = selected_project_id {
        get_applicability_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_applicability_by_project_cached(first_project.project_id)
        } else {
            get_applicability_all_cached()
        }
    };
    let applicability_json = json!(applicability.unwrap_or_default());

    let ctx = json!({
        "categories": categories_json, 
        "status": status_json, 
        "parent": parents_json, 
        "users": users_json, 
        "verification": verification_json, 
        "applicability": applicability_json,
        "selected_project_id": selected_project_id.unwrap_or(1),
        "req_title": "",
        "req_description": "",
        "req_justification": "",
        "req_reference": "",
        "req_link": "",
        "user": user
    });

    Ok(Template::render("new_requirement", ctx))
}

#[post("/new_requirement", data = "<new_req>")]
pub fn post_requirement(new_req: Form<NewRequirement>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let user = require_auth(cookies)?;
    let connection = &mut get_db_connection().map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(new_requirement))
    })?;
    
    let mut requirement_data = new_req.into_inner();
    
    // Server-side validation: Check if reference matches category
    if !requirement_data.req_reference.is_empty() {
        // Get the category to validate the reference
        let category = get_category_by_id(requirement_data.req_category);
        let expected_prefix = format!("REQ-{}-", category.cat_tag);
        
        if !requirement_data.req_reference.starts_with(&expected_prefix) {
            // Invalid reference format - redirect back to form with error
            return Err(Redirect::to(uri!(new_requirement)));
        }
        
        // Validate format: REQ-TAG-NUMBER
        let reference_pattern = format!("^REQ-{}-\\d+$", category.cat_tag);
        let regex = match Regex::new(&reference_pattern) {
            Ok(pattern) => pattern,
            Err(_) => {
                eprintln!("Failed to compile regex pattern: {}", reference_pattern);
                return Err(Redirect::to(uri!(new_requirement)));
            }
        };
        if !regex.is_match(&requirement_data.req_reference) {
            // Invalid reference format - redirect back to form with error
            return Err(Redirect::to(uri!(new_requirement)));
        }
    }
    
    // Generate automatic reference code if not provided
    if requirement_data.req_reference.is_empty() {
        match generate_requirement_reference(requirement_data.req_category, requirement_data.project_id) {
            Ok(reference) => {
                requirement_data.req_reference = reference;
            }
            Err(_e) => {
                // If generation fails, use a fallback reference
                requirement_data.req_reference = format!("REQ-UNKNOWN-{}", chrono::Utc::now().timestamp());
            }
        }
    }
    
    let my_id = insert_new_requirement(connection, &requirement_data)
        .map_err(|e| {
            eprintln!("Error inserting new requirement: {:?}", e);
            Redirect::to(uri!(show_requirements(None::<i32>, None::<i32>, None::<i32>)))
        })?;

    // Log the requirement creation
            if let Ok(new_values) = Logger::to_json_string(&requirement_data) {
        let _ = Logger::log_create(
            connection,
            user.user_id,
            EntityType::Requirement,
            my_id,
            Some(requirement_data.project_id),
            Some(new_values),
            Some(format!("Created requirement: {}", requirement_data.req_title)),
            None,
        );
    }

    // Invalidate cache for the new requirement
    invalidate_requirement_cache_complete(my_id);

    Ok(Redirect::to(uri!(show_requirement_id(my_id))))
}

#[get("/tests?<status_filter>&<verification_filter>&<category_filter>")]
pub fn show_tests(
    cookies: &CookieJar<'_>,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let mut ctx = build_context_with_projects(user, cookies);
    
    // Get selected project ID
    let selected_project_id = get_selected_project_id(cookies);
    
    let tests = if let Some(project_id) = selected_project_id {
        get_tests_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_tests_by_project_cached(first_project.project_id)
        } else {
            get_tests_all_cached()
        }
    };
    
    let tests_data = tests.unwrap_or_default();
    // Apply filters
    let filtered_tests = filter_tests(tests_data, status_filter, verification_filter, category_filter);
    let tests_decorate = decorate_tests(filtered_tests);
    ctx["tests"] = json!(tests_decorate);

    // Add filter data to context for the template
    let statuses = get_status_all_cached().unwrap_or_default();
    let verifications = if let Some(project_id) = selected_project_id {
        get_verification_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_verification_by_project_cached(first_project.project_id)
        } else {
            get_verification_all_cached()
        }
    };
    
    // Get categories filtered by selected project
    let categories = if let Some(project_id) = selected_project_id {
        get_categories_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_categories_by_project_cached(first_project.project_id)
        } else {
            get_categories_all_cached()
        }
    };
    
    ctx["statuses"] = json!(statuses);
    ctx["verifications"] = json!(verifications.unwrap_or_default());
    ctx["categories"] = json!(categories.unwrap_or_default());
    ctx["current_status_filter"] = json!(status_filter);
    ctx["current_verification_filter"] = json!(verification_filter);
    ctx["current_category_filter"] = json!(category_filter);

    Ok(Template::render("tests", ctx))
}

#[get("/tests/<test_id_param>")]
pub fn show_test_id(test_id_param: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Use the safe function that returns a Result
    match get_test_by_id_cached_safe(test_id_param) {
        Ok(test) => {
            let test_decorate = decorate_tests(vec![test]);
            
            // Get linked requirements for this test
            let linked_requirements = get_requirements_for_test(test_id_param).unwrap_or_default();
            let linked_requirements_json = json!(linked_requirements);
            
            let decorated_test = &test_decorate[0];
            let ctx = json!({
                "test_id": decorated_test.test_id,
                "test_name": decorated_test.test_name,
                "test_description": decorated_test.test_description,
                "test_source": decorated_test.test_source,
                "test_status": decorated_test.test_status,
                "test_parent_id": decorated_test.test_parent_id,
                "test_parent_title": decorated_test.test_parent_title,
                "linked_requirements": linked_requirements_json,
                "user": user
            });

            Ok(Template::render("test_by_id", ctx))
        },
        Err(error_msg) => {
            // Render error template instead of panicking
            let ctx = json!({
                "title": "Test Not Found",
                "message": "The test you're looking for could not be found.",
                "details": error_msg,
                "user": user
            });
            
            Ok(Template::render("error", ctx))
        }
    }
}

#[get("/new_test")]
pub fn new_test(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let status = get_status_all_cached().unwrap_or_default();
    let status_json = json!(status);

    // Get selected project ID and filter categories accordingly
    let selected_project_id = get_selected_project_id(cookies);
    let categories = if let Some(project_id) = selected_project_id {
        get_categories_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_categories_by_project_cached(first_project.project_id)
        } else {
            get_categories_all_cached()
        }
    };
    let categories_json = json!(categories.unwrap_or_default());

    // Get parent tests filtered by project
    let parents = if let Some(project_id) = selected_project_id {
        get_tests_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_tests_by_project_cached(first_project.project_id)
        } else {
            get_tests_all_cached()
        }
    };
    let parents_json = json!(parents.unwrap_or_default());

    let users = get_users_all_cached().unwrap_or_default();
    let users_json = json!(users);

    // Get requirements filtered by project
    let requirements = if let Some(project_id) = selected_project_id {
        get_requirements_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_requirements_by_project_cached(first_project.project_id)
        } else {
            get_requirements_all_cached()
        }
    };
    let requirements_json = json!(requirements.unwrap_or_default());

    let ctx = json!({
        "categories": categories_json, 
        "status": status_json, 
        "parents": parents_json, 
        "users": users_json, 
        "requirements": requirements_json,
        "user": user
    });

    Ok(Template::render("new_test", ctx))
}

#[get("/edit_test/<test_id>")]
pub fn get_edit_test(test_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let test = get_test_by_id(test_id);
    let test_decorate = decorate_tests(vec![test]);
    let test_decorate_json = json!(test_decorate[0]);

    let status = get_status_all_cached().unwrap_or_default();
    let status_json = json!(status);

    // Get selected project ID and filter categories accordingly
    let selected_project_id = get_selected_project_id(cookies);
    let categories = if let Some(project_id) = selected_project_id {
        get_categories_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_categories_by_project_cached(first_project.project_id)
        } else {
            get_categories_all_cached()
        }
    };
    let categories_json = json!(categories.unwrap_or_default());

    // Get parent tests filtered by project
    let parents = if let Some(project_id) = selected_project_id {
        get_tests_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_tests_by_project_cached(first_project.project_id)
        } else {
            get_tests_all_cached()
        }
    };
    let parents_json = json!(parents.unwrap_or_default());

    let users = get_users_all_cached().unwrap_or_default();
    let users_json = json!(users);

    // Get verification types filtered by project
    let verification_types = if let Some(project_id) = selected_project_id {
        get_verification_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_verification_by_project_cached(first_project.project_id)
        } else {
            get_verification_all_cached()
        }
    };
    let verification_json = json!(verification_types.unwrap_or_default());

    // Get linked requirements for this test
    let linked_requirements = get_requirements_for_test_cached(test_id).unwrap_or_default();
    let linked_requirements_json = json!(linked_requirements);

    // Create a simple array of linked requirement IDs for template checking
    let linked_req_ids: Vec<i32> = linked_requirements.iter().map(|r| r.req_id).collect();
    let linked_req_ids_json = json!(linked_req_ids);

    // Get all requirements for the multi-select (filtered by project)
    let all_requirements = if let Some(project_id) = selected_project_id {
        get_requirements_by_project_cached(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_requirements_by_project_cached(first_project.project_id)
        } else {
            get_requirements_all_cached()
        }
    };
    let all_requirements_json = json!(all_requirements.unwrap_or_default());

    let ctx = json!({
        "tests": test_decorate_json, 
        "categories": categories_json, 
        "status": status_json, 
        "parent": parents_json, 
        "users": users_json, 
        "verification": verification_json,
        "linked_requirements": linked_requirements_json,
        "linked_req_ids": linked_req_ids_json,
        "requirements": all_requirements_json,
        "user": user
    });

    #[cfg(debug_assertions)]
    println!("Tests: {:#}", ctx);
    Ok(Template::render("edit_test_by_id", ctx))
}

#[allow(unused_variables)]
#[post("/edit_test/<test_id>", data = "<edit_test_form>")]
pub fn post_edit_test(test_id: i32, edit_test_form: Form<EditTestForm>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let user = require_auth(cookies)?;
    let connection = &mut get_db_connection().map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(get_edit_test(test_id)))
    })?;
    
    // Get the old values before updating
    let old_test = get_test_by_id(test_id);
    
    // First, update the test details
    let new_test = NewTest {
        test_id: Some(edit_test_form.test_id),
        test_name: edit_test_form.test_name.clone(),
        test_description: edit_test_form.test_description.clone(),
        test_source: edit_test_form.test_source.clone(),
        test_status: edit_test_form.test_status,
        test_parent: edit_test_form.test_parent,
        project_id: edit_test_form.project_id,
    };
    
    edit_test(connection, &new_test)
        .map_err(|e| {
            eprintln!("Error editing test: {:?}", e);
            Redirect::to(uri!(show_tests(None::<i32>, None::<i32>, None::<i32>)))
        })?;
    
    // Log the test update
            if let (Ok(old_values), Ok(new_values)) = (Logger::to_json_string(&old_test), Logger::to_json_string(&new_test)) {
        let _ = Logger::log_update(
            connection,
            user.user_id,
            EntityType::Test,
            test_id,
            Some(edit_test_form.project_id),
            Some(old_values),
            Some(new_values),
            Some(format!("Updated test: {}", new_test.test_name)),
            None,
        );
    }
    
    // Then, update the requirement links
    update_test_requirement_links(connection, edit_test_form.test_id, &edit_test_form.linked_requirements)
        .map_err(|e| {
            eprintln!("Error updating test requirement links: {:?}", e);
            Redirect::to(uri!(show_tests(None::<i32>, None::<i32>, None::<i32>)))
        })?;

    // Invalidate cache for the updated test
    invalidate_test_cache_complete(test_id);

    Ok(Redirect::to(uri!(show_test_id(edit_test_form.test_id))))
}

#[post("/new_test", data = "<new_test>")]
pub fn post_test(new_test: Form<NewTestForm>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let user = require_auth(cookies)?;
    let connection = &mut get_db_connection().map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(new_test))
    })?;
    let my_new_test = NewTest {
        test_id: None,
        test_name: new_test.test_name.clone(),
        test_description: new_test.test_description.clone(),
        test_source: new_test.test_source.clone(),
        test_status: new_test.test_status,
        test_parent: new_test.test_parent,
        project_id: new_test.project_id,
    };
    let my_id = insert_new_test(connection, &my_new_test)
        .map_err(|e| {
            eprintln!("Error inserting new test: {:?}", e);
            Redirect::to(uri!(show_tests(None::<i32>, None::<i32>, None::<i32>)))
        })?;

    // Log the test creation
            if let Ok(new_values) = Logger::to_json_string(&my_new_test) {
        let _ = Logger::log_create(
            connection,
            user.user_id,
            EntityType::Test,
            my_id,
            Some(new_test.project_id),
            Some(new_values),
            Some(format!("Created test: {}", my_new_test.test_name)),
            None,
        );
    }

    #[cfg(debug_assertions)]
    println!("NewTestForm requirements: {:#?}", new_test.test_req);
    for req in new_test.test_req.iter() {
        let matrix_item = NewMatrix {
            matrix_req_id: *req,
            matrix_test_id: my_id,
            project_id: new_test.project_id,
        };
        insert_new_matrix_item(connection, &matrix_item)
            .map_err(|e| {
                eprintln!("Error inserting matrix item: {:?}", e);
                Redirect::to(uri!(show_tests(None::<i32>, None::<i32>, None::<i32>)))
            })?;
    }

    // Invalidate cache for the new test
    invalidate_test_cache_complete(my_id);

    Ok(Redirect::to(uri!(show_test_id(my_id))))
}

#[get("/status")]
pub fn show_status() -> content::RawHtml<String> {
    use crate::schema::status::dsl::*;

    let mut out_str = print_header();
    let mut connection = match get_db_connection() {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return content::RawHtml("Error: Database connection failed".to_string());
        }
    };

    let all_status = match status.load::<Status>(connection.as_mut()) {
        Ok(status_list) => status_list,
        Err(e) => {
            eprintln!("Database query error: {}", e);
            return content::RawHtml("Error: Failed to load status data".to_string());
        }
    };

    for st in all_status.iter() {
        out_str = format!(
            "{}
        <div class='AllStatus'>
            <div>Id: {}</div>
            <div>Title: {}</div>
            <div>Description: {}</div>
        </div>",
            out_str, st.st_id, st.st_title, st.st_description
        );
    }

    out_str = format!("{} {}", out_str, print_footer());
    content::RawHtml(out_str)
}

#[get("/matrix?<sort_by>&<sort_order>&<test_status_filter>")]
pub fn get_matrix(cookies: &CookieJar<'_>, sort_by: Option<String>, sort_order: Option<String>, test_status_filter: Option<i32>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    use crate::schema::matrix::dsl::*;
    use crate::schema::requirements::dsl::*;
    use crate::schema::tests::dsl::*;

    let mut connection = match get_db_connection() {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(Redirect::to(uri!(index)));
        }
    };

    // Get selected project ID
    let selected_project_id = get_selected_project_id(cookies);
    
    let mut all_reqs = if let Some(selected_pid) = selected_project_id {
        requirements
            .filter(crate::schema::requirements::project_id.eq(selected_pid))
            .load::<Requirement>(connection.as_mut())
            .map_err(|e| {
                eprintln!("Database connection error: {}", e);
                "Error querying requirements from the database".to_string()
            })
            .expect("Error getting matrix table")
    } else {
        requirements
            .load::<Requirement>(connection.as_mut())
            .map_err(|e| {
                eprintln!("Database connection error: {}", e);
                "Error querying page views from the database".to_string()
            })
            .expect("Error getting matrix table")
    };

    let mut all_tests = if let Some(selected_pid) = selected_project_id {
        tests
            .filter(crate::schema::tests::project_id.eq(selected_pid))
            .load::<Test>(connection.as_mut())
            .map_err(|e| {
                eprintln!("Database connection error: {}", e);
                "Error querying tests from the database".to_string()
            })
            .expect("Error getting tests")
    } else {
        tests
            .load::<Test>(connection.as_mut())
            .map_err(|e| {
                eprintln!("Database connection error: {}", e);
                "Error querying tests from the database".to_string()
            })
            .expect("Error getting tests")
    };

    // Always sort tests by test_id (number)
    all_tests.sort_by(|a, b| a.test_id.cmp(&b.test_id));

    // Filter tests by status if filter is provided
    if let Some(status_filter) = test_status_filter {
        all_tests.retain(|test| test.test_status == status_filter);
    }

    // Apply sorting
    let sort_by = sort_by.unwrap_or_else(|| "req_id".to_string());
    let sort_order = sort_order.unwrap_or_else(|| "asc".to_string());
    
    // Check if sorting by test column
    if sort_by.starts_with("test_") {
        // Extract test ID from sort_by (e.g., "test_1" -> test_id = 1)
        if let Ok(target_test_id) = sort_by.trim_start_matches("test_").parse::<i32>() {
            // Sort requirements based on their link status to the specified test
            if sort_order == "desc" {
                all_reqs.sort_by(|a, b| {
                    let a_has_link: i64 = matrix
                        .filter(matrix_req_id.eq(a.req_id))
                        .filter(matrix_test_id.eq(target_test_id))
                        .count()
                        .get_result(connection.as_mut())
                        .unwrap();
                    let b_has_link: i64 = matrix
                        .filter(matrix_req_id.eq(b.req_id))
                        .filter(matrix_test_id.eq(target_test_id))
                        .count()
                        .get_result(connection.as_mut())
                        .unwrap();
                    b_has_link.cmp(&a_has_link)
                });
            } else {
                all_reqs.sort_by(|a, b| {
                    let a_has_link: i64 = matrix
                        .filter(matrix_req_id.eq(a.req_id))
                        .filter(matrix_test_id.eq(target_test_id))
                        .count()
                        .get_result(connection.as_mut())
                        .unwrap();
                    let b_has_link: i64 = matrix
                        .filter(matrix_req_id.eq(b.req_id))
                        .filter(matrix_test_id.eq(target_test_id))
                        .count()
                        .get_result(connection.as_mut())
                        .unwrap();
                    a_has_link.cmp(&b_has_link)
                });
            }
        }
    } else {
        // Sort requirements by requirement fields
        match sort_by.as_str() {
            "req_id" => {
                if sort_order == "desc" {
                    all_reqs.sort_by(|a, b| b.req_id.cmp(&a.req_id));
                } else {
                    all_reqs.sort_by(|a, b| a.req_id.cmp(&b.req_id));
                }
            }
            "req_title" => {
                if sort_order == "desc" {
                    all_reqs.sort_by(|a, b| b.req_title.cmp(&a.req_title));
                } else {
                    all_reqs.sort_by(|a, b| a.req_title.cmp(&b.req_title));
                }
            }
            "req_reference" => {
                if sort_order == "desc" {
                    all_reqs.sort_by(|a, b| b.req_reference.cmp(&a.req_reference));
                } else {
                    all_reqs.sort_by(|a, b| a.req_reference.cmp(&b.req_reference));
                }
            }
            _ => {
                // Default sort by req_id ascending
                all_reqs.sort_by(|a, b| a.req_id.cmp(&b.req_id));
            }
        }
    }

    let total_tests = all_tests.len() as i32;
    let total_requirements = all_reqs.len() as i32;

    // Create matrix data structure
    let mut total_links = 0;
    let mut requirements_with_matrix = Vec::new();

    for req in &all_reqs {
        let mut req_matrix = Vec::new();
        
        for test in &all_tests {
            let test_present: i64 = matrix
                .filter(matrix_req_id.eq(req.req_id))
                .filter(matrix_test_id.eq(test.test_id))
                .count()
                .get_result(connection.as_mut())
                .unwrap();

            if test_present > 0 {
                req_matrix.push(json!({
                    "linked": true,
                    "test_status": test.test_status
                }));
                total_links += 1;
            } else {
                req_matrix.push(json!({
                    "linked": false,
                    "test_status": null
                }));
            }
        }
        
        requirements_with_matrix.push(json!({
            "req_id": req.req_id,
            "req_title": req.req_title,
            "req_reference": req.req_reference,
            "matrix": req_matrix
        }));
    }

    // Prepare tests with status names
    let mut tests_with_status = Vec::new();
    for test in all_tests {
        let test_status_name = get_status_name_by_id(test.test_status);
        tests_with_status.push(json!({
            "test_id": test.test_id,
            "test_name": test.test_name,
            "test_status": test_status_name
        }));
    }

    // Get all statuses for the filter dropdown
    let all_statuses = get_status_all_cached().unwrap_or_default();
    let statuses_json = json!(all_statuses);

    let mut ctx = build_context_with_projects(user, cookies);
    ctx["requirements"] = json!(requirements_with_matrix);
    ctx["tests"] = json!(tests_with_status);
    ctx["total_tests"] = json!(total_tests);
    ctx["total_requirements"] = json!(total_requirements);
    ctx["total_links"] = json!(total_links);
    ctx["current_sort_by"] = json!(sort_by);
    ctx["current_sort_order"] = json!(sort_order);
    ctx["test_status_filter"] = json!(test_status_filter);
    ctx["statuses"] = json!(statuses_json);

    Ok(Template::render("matrix", ctx))
}

#[get("/matrix.xls")]
pub async fn get_matrix_xls(cookies: &CookieJar<'_>) -> Result<(ContentType, NamedFile), Redirect> {
    let _user = require_auth(cookies)?;
    
    match excel::create_matrix_workbook(cookies) {
        Ok(_) => {
            let path_to_file = path::Path::new("target/matrix.xls");
            let res = NamedFile::open(&path_to_file)
                .await
                .map_err(|e| NotFound(e.to_string()));
            match res {
                Ok(file) => {
                    let content_type = ContentType::new(
                        "application",
                        "vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                    );
                    Ok((content_type, file))
                }
                Err(error) => {
                    eprintln!("Error opening matrix file: {:?}", error);
                    Err(Redirect::to("/matrix"))
                }
            }
        }
        Err(e) => {
            eprintln!("Error creating matrix workbook: {:?}", e);
            Err(Redirect::to("/matrix"))
        }
    }
}

#[get("/requirements.xls")]
pub async fn get_requirements_xls(cookies: &CookieJar<'_>) -> Result<(ContentType, NamedFile), Redirect> {
    let _user = require_auth(cookies)?;
    let _file = excel::create_requirements_workbook().expect("file can be created");
    let path_to_file = path::Path::new("target/requirements.xls");
    let res = NamedFile::open(&path_to_file)
        .await
        .map_err(|e| NotFound(e.to_string()));
    match res {
        Ok(file) => {
            let content_type = ContentType::new(
                "application",
                "vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            );
            Ok((content_type, file))
        }

        Err(error) => panic!("Problem with file {:?}", error),
    }
}

#[get("/tests.xls")]
pub async fn get_tests_xls(cookies: &CookieJar<'_>) -> Result<(ContentType, NamedFile), Redirect> {
    let _user = require_auth(cookies)?;
    let _file = excel::create_tests_workbook().expect("file can be created");
    let path_to_file = path::Path::new("target/tests.xls");
    let res = NamedFile::open(&path_to_file)
        .await
        .map_err(|e| NotFound(e.to_string()));
    match res {
        Ok(file) => {
            let content_type = ContentType::new(
                "application",
                "vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            );
            Ok((content_type, file))
        }

        Err(error) => panic!("Problem with file {:?}", error),
    }
}

#[get("/new_user")]
pub fn new_user(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let status = get_status_all_cached().unwrap_or_default();
    let status_json = json!(status);

    let ctx = json!({
        "status": status_json,
        "user": user
    });
    Ok(Template::render("new_user", ctx))
}

#[get("/categories")]
pub fn show_categories(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let mut ctx = build_context_with_projects(user, cookies);
    
    // Get selected project ID
    let selected_project_id = get_selected_project_id(cookies);
    
    let categories = if let Some(project_id) = selected_project_id {
        get_categories_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_categories_by_project(first_project.project_id)
        } else {
            get_categories_all_cached()
        }
    };

    match categories {
        Ok(cats) => {
            ctx["categories"] = json!(cats);
        }
        Err(_) => {
            ctx["categories"] = json!([]);
        }
    };

    Ok(Template::render("categories", ctx))
}

#[get("/new_category")]
pub fn new_category(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Get projects and selected project
    let projects = get_projects_for_nav_cached().unwrap_or_default();
    let mut selected_project_id = get_selected_project_id(cookies);
    
    // If no project is selected and there are projects available, select the first one
    if selected_project_id.is_none() && !projects.is_empty() {
        selected_project_id = Some(projects[0].project_id);
        // Set the cookie for the selected project
        cookies.add(Cookie::new("selected_project_id", projects[0].project_id.to_string()));
    }
    
    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": selected_project_id
    });
    Ok(Template::render("new_category", ctx))
}

#[post("/new_category", data = "<new_category>")]
pub fn post_category(new_category: Form<NewCategory>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if project_id is provided
    if new_category.project_id == 0 {
        return Ok(Redirect::to(uri!(new_category)));
    }
    
    let connection = &mut get_db_connection().map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(new_category))
    })?;
    
    let category_data = new_category.into_inner();
    let result = insert_new_category(connection, &category_data);
    match result {
        Ok(category_id) => {
            // Log the category creation
            if let Ok(new_values) = Logger::to_json_string(&category_data) {
                let _ = Logger::log_create(
                    connection,
                    user.user_id,
                    EntityType::Category,
                    category_id,
                    Some(category_data.project_id),
                    Some(new_values),
                    Some(format!("Created category: {}", category_data.cat_title)),
                    None,
                );
            }
            
            // Invalidate cache for the new category
            invalidate_category_cache_complete(category_id);
            
            Ok(Redirect::to(uri!(show_categories)))
        },
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(new_category)))
        }
    }
}

#[get("/edit_category/<cat_id>")]
pub fn get_edit_category(cat_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let category = get_category_by_id_cached(cat_id);
    let ctx = json!({
        "categories": category,
        "user": user
    });
    Ok(Template::render("edit_category", ctx))
}

#[post("/edit_category/<cat_id>", data = "<category>")]
pub fn post_edit_category(cat_id: i32, category: Form<NewCategory>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let user = require_auth(cookies)?;
    let connection = &mut get_db_connection().map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(get_edit_category(cat_id)))
    })?;
    
    // Get the old values before updating
    let old_category = get_category_by_id(cat_id);
    
    let mut category_with_id = category.into_inner();
    category_with_id.cat_id = Some(cat_id);
    
    let result = edit_category(connection, &category_with_id);
    match result {
        Ok(_) => {
            // Log the category update
            if let (Ok(old_values), Ok(new_values)) = (Logger::to_json_string(&old_category), Logger::to_json_string(&category_with_id)) {
                let _ = Logger::log_update(
                    connection,
                    user.user_id,
                    EntityType::Category,
                    cat_id,
                    Some(category_with_id.project_id),
                    Some(old_values),
                    Some(new_values),
                    Some(format!("Updated category: {}", category_with_id.cat_title)),
                    None,
                );
            }
            
            // Invalidate cache for the updated category
            invalidate_category_cache_complete(cat_id);
            
            Ok(Redirect::to(uri!(show_categories)))
        },
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(get_edit_category(cat_id))))
        }
    }
}

#[delete("/delete_category/<cat_id>")]
pub fn delete_category_route(cat_id: i32, cookies: &CookieJar<'_>) -> Result<rocket::http::Status, Redirect> {
    let user = require_auth(cookies)?;
    let mut connection = match get_db_connection() {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(Redirect::to(uri!(show_categories)));
        }
    };
    
    // Get the category details before deleting
    let category = get_category_by_id(cat_id);
    
    let result = delete_category(connection.as_mut(), &cat_id);
    match result {
        Ok(_) => {
            // Log the category deletion
            if let Ok(old_values) = Logger::to_json_string(&category) {
                let _ = Logger::log_delete(
                    connection.as_mut(),
                    user.user_id,
                    EntityType::Category,
                    cat_id,
                    Some(category.project_id),
                    Some(old_values),
                    Some(format!("Deleted category: {}", category.cat_title)),
                    None,
                );
            }
            
            // Invalidate cache for the deleted category
            invalidate_category_cache_complete(cat_id);
            
            Ok(rocket::http::Status::Ok)
        },
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(rocket::http::Status::InternalServerError)
        }
    }
}

#[post("/new_user", data = "<new_user>")]
pub fn post_user(new_user: Form<NewUser>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let user = require_auth(cookies)?;
    let connection = &mut get_db_connection().map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(new_user))
    })?;
    
    // Hash the password before inserting
    let mut user_with_hashed_password = new_user.into_inner();
    match hash_password(&user_with_hashed_password.user_password) {
        Ok(hashed_password) => {
            user_with_hashed_password.user_password = hashed_password;
            let my_id = insert_new_user(connection, &user_with_hashed_password)
                .map_err(|e| {
                    eprintln!("Error inserting new user: {:?}", e);
                    Redirect::to(uri!(new_user))
                })?;
            
            // Log the user creation
            if let Ok(new_values) = Logger::to_json_string(&user_with_hashed_password) {
                let _ = Logger::log_create(
                    connection,
                    user.user_id,
                    EntityType::User,
                    my_id,
                    None,
                    Some(new_values),
                    Some(format!("Created user: {}", user_with_hashed_password.user_username)),
                    None,
                );
            }
            
            // Invalidate cache for the new user
            invalidate_user_cache_complete(my_id);
            
            Ok(Redirect::to(uri!(show_user_id(my_id))))
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(new_user)))
        }
    }
}

#[get("/applicability")]
pub fn show_applicability(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let mut ctx = build_context_with_projects(user, cookies);
    
    // Get selected project ID
    let selected_project_id = get_selected_project_id(cookies);
    
    let applicability = if let Some(project_id) = selected_project_id {
        get_applicability_by_project(project_id)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            get_applicability_by_project(first_project.project_id)
        } else {
            get_applicability_all()
        }
    };

    match applicability {
        Ok(apps) => {
            ctx["applicability"] = json!(apps);
        }
        Err(_) => {
            ctx["applicability"] = json!([]);
        }
    };

    Ok(Template::render("applicability", ctx))
}

#[get("/new_applicability")]
pub fn new_applicability(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Get projects and selected project
    let projects = get_projects_for_nav().unwrap_or_default();
    let mut selected_project_id = get_selected_project_id(cookies);
    
    // If no project is selected and there are projects available, select the first one
    if selected_project_id.is_none() && !projects.is_empty() {
        selected_project_id = Some(projects[0].project_id);
        // Set the cookie for the selected project
        cookies.add(Cookie::new("selected_project_id", projects[0].project_id.to_string()));
    }
    
    let ctx = json!({
        "user": user,
        "projects": projects,
        "selected_project_id": selected_project_id
    });
    Ok(Template::render("new_applicability", ctx))
}

#[post("/new_applicability", data = "<new_applicability>")]
pub fn post_applicability(new_applicability: Form<NewApplicability>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if project_id is provided
    if new_applicability.project_id == 0 {
        return Ok(Redirect::to(uri!(new_applicability)));
    }
    
    let connection = &mut get_db_connection().map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(new_applicability))
    })?;
    
    let applicability_data = new_applicability.into_inner();
    let result = insert_new_applicability(connection, &applicability_data);
    match result {
        Ok(applicability_id) => {
            // Log the applicability creation
            if let Ok(new_values) = Logger::to_json_string(&applicability_data) {
                let _ = Logger::log_create(
                    connection,
                    user.user_id,
                    EntityType::Applicability,
                    applicability_id,
                    Some(applicability_data.project_id),
                    Some(new_values),
                    Some(format!("Created applicability: {}", applicability_data.app_title)),
                    None,
                );
            }
            
            // Invalidate cache for the new applicability
            invalidate_applicability_cache_complete(applicability_id);
            
            Ok(Redirect::to(uri!(show_applicability)))
        },
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(new_applicability)))
        }
    }
}

#[get("/edit_applicability/<app_id>")]
pub fn get_edit_applicability(app_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let applicability = get_applicability_by_id(app_id);
    let ctx = json!({
        "applicability": applicability,
        "user": user
    });
    Ok(Template::render("edit_applicability", ctx))
}

#[post("/edit_applicability/<app_id>", data = "<applicability>")]
pub fn post_edit_applicability(app_id: i32, applicability: Form<NewApplicability>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let user = require_auth(cookies)?;
    let connection = &mut get_db_connection().map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(get_edit_applicability(app_id)))
    })?;
    
    // Get the old values before updating
    let old_applicability = get_applicability_by_id(app_id);
    
    let mut applicability_with_id = applicability.into_inner();
    applicability_with_id.app_id = Some(app_id);
    
    let result = edit_applicability(connection, &applicability_with_id);
    match result {
        Ok(_) => {
            // Log the applicability update
            if let (Ok(old_values), Ok(new_values)) = (Logger::to_json_string(&old_applicability), Logger::to_json_string(&applicability_with_id)) {
                let _ = Logger::log_update(
                    connection,
                    user.user_id,
                    EntityType::Applicability,
                    app_id,
                    Some(applicability_with_id.project_id),
                    Some(old_values),
                    Some(new_values),
                    Some(format!("Updated applicability: {}", applicability_with_id.app_title)),
                    None,
                );
            }
            Ok(Redirect::to(uri!(show_applicability)))
        },
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(get_edit_applicability(app_id))))
        }
    }
}

#[delete("/delete_applicability/<app_id>")]
pub fn delete_applicability_route(app_id: i32, cookies: &CookieJar<'_>) -> Result<rocket::http::Status, Redirect> {
    let user = require_auth(cookies)?;
    let mut connection = match get_db_connection() {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(Redirect::to(uri!(show_applicability)));
        }
    };
    
    // Get the applicability details before deleting
    let applicability = get_applicability_by_id(app_id);
    
    let result = delete_applicability(connection.as_mut(), &app_id);
    match result {
        Ok(_) => {
            // Log the applicability deletion
            if let Ok(old_values) = Logger::to_json_string(&applicability) {
                let _ = Logger::log_delete(
                    connection.as_mut(),
                    user.user_id,
                    EntityType::Applicability,
                    app_id,
                    Some(applicability.project_id),
                    Some(old_values),
                    Some(format!("Deleted applicability: {}", applicability.app_title)),
                    None,
                );
            }
            
            // Invalidate cache for the deleted applicability
            crate::cached_functions::invalidate_applicability_cache_complete(app_id);
            
            Ok(rocket::http::Status::Ok)
        },
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(rocket::http::Status::InternalServerError)
        }
    }
}

#[get("/requirements/tree")]
pub fn show_requirements_tree(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Get all requirements
    let all_requirements = get_requirements_all_cached().unwrap_or_default();
    
    // Build tree structure
    let mut tree_data = Vec::new();
    let mut children_map: std::collections::HashMap<i32, Vec<&Requirement>> = std::collections::HashMap::new();
    
    // Group requirements by parent
    for req in &all_requirements {
        if req.req_parent == 0 {
            // Root requirements
            tree_data.push(req);
        } else {
            // Child requirements
            children_map.entry(req.req_parent).or_insert_with(Vec::new).push(req);
        }
    }
    
    // Sort requirements by ID
    tree_data.sort_by(|a, b| a.req_id.cmp(&b.req_id));
    
    // Create tree structure with children
    let mut tree_structure = Vec::new();
    for root_req in tree_data {
        let mut node = json!({
            "requirement": root_req,
            "children": Vec::<serde_json::Value>::new()
        });
        
        // Add children if any
        if let Some(children) = children_map.get(&root_req.req_id) {
            let mut sorted_children = children.clone();
            sorted_children.sort_by(|a, b| a.req_id.cmp(&b.req_id));
            
            let children_json: Vec<serde_json::Value> = sorted_children
                .iter()
                .map(|child| json!({
                    "requirement": child,
                    "children": Vec::<serde_json::Value>::new()
                }))
                .collect();
            
            node["children"] = json!(children_json);
        }
        
        tree_structure.push(node);
    }
    
    let ctx = json!({
        "tree_data": tree_structure,
        "total_requirements": all_requirements.len(),
        "user": user
    });
    
    Ok(Template::render("requirements_tree", ctx))
}

#[get("/reports")]
pub fn show_reports(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Get selected project ID
    let selected_project_id = get_selected_project_id(cookies);
    
    // Get project-specific data for metrics
    let (all_requirements, all_tests, all_categories) = if let Some(project_id) = selected_project_id {
        let requirements = get_requirements_by_project(project_id).unwrap_or_default();
        let tests = get_tests_by_project(project_id).unwrap_or_default();
        let categories = get_categories_by_project(project_id).unwrap_or_default();
        (requirements, tests, categories)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            let requirements = get_requirements_by_project(first_project.project_id).unwrap_or_default();
            let tests = get_tests_by_project(first_project.project_id).unwrap_or_default();
            let categories = get_categories_by_project(first_project.project_id).unwrap_or_default();
            (requirements, tests, categories)
        } else {
            // Fallback to all data if no projects exist
            (get_requirements_all_cached().unwrap_or_default(), 
             get_tests_all_cached().unwrap_or_default(), 
             get_categories_all_cached().unwrap_or_default())
        }
    };
    
    let all_users = get_users_all_cached().unwrap_or_default();
    let all_statuses = get_status_all_cached().unwrap_or_default();
    
    // Calculate metrics
    let total_requirements = all_requirements.len();
    let total_tests = all_tests.len();
    let total_categories = all_categories.len();
    let total_users = all_users.len();
    
    // Requirements by status
    let mut requirements_by_status = std::collections::HashMap::new();
    for req in &all_requirements {
        let status_name = get_status_name_by_id(req.req_current_status);
        *requirements_by_status.entry(status_name).or_insert(0) += 1;
    }
    
    // Tests by status
    let mut tests_by_status = std::collections::HashMap::new();
    for test in &all_tests {
        let status_name = get_status_name_by_id(test.test_status);
        *tests_by_status.entry(status_name).or_insert(0) += 1;
    }
    
    // Requirements by category
    let mut requirements_by_category = std::collections::HashMap::new();
    for req in &all_requirements {
        let category = get_category_by_id(req.req_category);
        let category_name = category.cat_title;
        *requirements_by_category.entry(category_name).or_insert(0) += 1;
    }
    
    // Coverage metrics
    let mut covered_requirements = 0;
    let mut total_links = 0;
    for req in &all_requirements {
        let links = get_requirements_for_test(req.req_id).unwrap_or_default();
        if !links.is_empty() {
            covered_requirements += 1;
        }
        total_links += links.len();
    }
    
    let coverage_percentage = if total_requirements > 0 {
        ((covered_requirements as f64 / total_requirements as f64) * 100.0 * 10.0).round() / 10.0
    } else {
        0.0
    };
    
    let avg_tests_per_requirement = if total_requirements > 0 {
        ((total_links as f64 / total_requirements as f64) * 10.0).round() / 10.0
    } else {
        0.0
    };
    
    // Recent activity (last 30 days)
    let now = chrono::Utc::now();
    let _thirty_days_ago = now - chrono::Duration::days(30);
    
    let mut recent_requirements = 0;
    let mut recent_tests = 0;
    
    for _req in &all_requirements {
        // For now, we'll use a placeholder since creation_date might not be available
        recent_requirements += 1; // Placeholder
    }
    
    for _test in &all_tests {
        // Assuming test has creation date - you might need to add this field
        // For now, we'll use a placeholder
        recent_tests += 1; // Placeholder
    }
    
    // Get selected project name for display
    let selected_project_name = if let Some(project_id) = selected_project_id {
        let project = get_project_by_id_pooled_safe(project_id);
        project.project_name
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all_cached().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            first_project.project_name.clone()
        } else {
            "All Projects".to_string()
        }
    };
    
    let ctx = json!({
        "user": user,
        "selected_project_name": selected_project_name,
        "metrics": {
            "total_requirements": total_requirements,
            "total_tests": total_tests,
            "total_categories": total_categories,
            "total_users": total_users,
            "coverage_percentage": coverage_percentage,
            "avg_tests_per_requirement": avg_tests_per_requirement,
            "covered_requirements": covered_requirements,
            "total_links": total_links,
            "recent_requirements": recent_requirements,
            "recent_tests": recent_tests
        },
        "requirements_by_status": requirements_by_status,
        "tests_by_status": tests_by_status,
        "requirements_by_category": requirements_by_category,
        "all_statuses": all_statuses,
        "all_categories": all_categories
    });
    
    Ok(Template::render("reports", ctx))
}

#[get("/reports/pdf")]
pub fn generate_pdf_report(cookies: &CookieJar<'_>) -> Result<(rocket::http::ContentType, Vec<u8>), Redirect> {
    let _user = require_auth(cookies)?;
    
    // Get selected project ID
    let selected_project_id = get_selected_project_id(cookies);
    
    // Get project-specific data for metrics
    let (all_requirements, all_tests, all_categories) = if let Some(project_id) = selected_project_id {
        let requirements = get_requirements_by_project(project_id).unwrap_or_default();
        let tests = get_tests_by_project(project_id).unwrap_or_default();
        let categories = get_categories_by_project(project_id).unwrap_or_default();
        (requirements, tests, categories)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            let requirements = get_requirements_by_project(first_project.project_id).unwrap_or_default();
            let tests = get_tests_by_project(first_project.project_id).unwrap_or_default();
            let categories = get_categories_by_project(first_project.project_id).unwrap_or_default();
            (requirements, tests, categories)
        } else {
            // Fallback to all data if no projects exist
            (get_requirements_all_cached().unwrap_or_default(), 
             get_tests_all_cached().unwrap_or_default(), 
             get_categories_all_cached().unwrap_or_default())
        }
    };
    
    let all_users = get_users_all_cached().unwrap_or_default();
    let _all_statuses = get_status_all_cached().unwrap_or_default();
    
    // Calculate the same metrics
    let total_requirements = all_requirements.len();
    let total_tests = all_tests.len();
    let total_categories = all_categories.len();
    let total_users = all_users.len();
    
    // Requirements by status
    let mut requirements_by_status = std::collections::HashMap::new();
    for req in &all_requirements {
        let status_name = get_status_name_by_id(req.req_current_status);
        *requirements_by_status.entry(status_name).or_insert(0) += 1;
    }
    
    // Tests by status
    let mut tests_by_status = std::collections::HashMap::new();
    for test in &all_tests {
        let status_name = get_status_name_by_id(test.test_status);
        *tests_by_status.entry(status_name).or_insert(0) += 1;
    }
    
    // Requirements by category
    let mut requirements_by_category = std::collections::HashMap::new();
    for req in &all_requirements {
        let category = get_category_by_id(req.req_category);
        let category_name = category.cat_title;
        *requirements_by_category.entry(category_name).or_insert(0) += 1;
    }
    
    // Coverage metrics
    let mut covered_requirements = 0;
    let mut total_links = 0;
    for req in &all_requirements {
        let links = get_requirements_for_test(req.req_id).unwrap_or_default();
        if !links.is_empty() {
            covered_requirements += 1;
        }
        total_links += links.len();
    }
    
    let coverage_percentage = if total_requirements > 0 {
        ((covered_requirements as f64 / total_requirements as f64) * 100.0 * 10.0).round() / 10.0
    } else {
        0.0
    };
    
    let avg_tests_per_requirement = if total_requirements > 0 {
        ((total_links as f64 / total_requirements as f64) * 10.0).round() / 10.0
    } else {
        0.0
    };
    
    // Generate HTML content
    let html_content = generate_pdf_content(
        total_requirements,
        total_tests,
        total_categories,
        total_users,
        coverage_percentage,
        avg_tests_per_requirement,
        covered_requirements,
        total_links,
        requirements_by_status.clone(),
        tests_by_status.clone(),
        requirements_by_category.clone()
    );
    
    // Generate PDF using the new PDF generation function
    match generate_pdf_report_data(
        total_requirements,
        total_tests,
        total_categories,
        total_users,
        coverage_percentage,
        avg_tests_per_requirement,
        covered_requirements,
        total_links,
        requirements_by_status,
        tests_by_status,
        requirements_by_category
    ) {
        Ok(pdf_bytes) => {
            let content_type = rocket::http::ContentType::new("application", "pdf");
            Ok((content_type, pdf_bytes))
        }
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("PDF generation failed: {:?}", _e);
            // Fallback to HTML if PDF generation fails
            let content_type = rocket::http::ContentType::new("text", "html");
            Ok((content_type, html_content.into_bytes()))
        }
    }
}

// Project management routes
#[get("/projects")]
pub fn show_projects(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let projects = get_projects_all();

    let ctx = match projects {
        Ok(projs) => {
            json!({
                "projects": projs,
                "user": user
            })
        }
        Err(_) => {
            json!({
                "projects": [],
                "user": user
            })
        }
    };

    Ok(Template::render("projects", ctx))
}

#[get("/projects/<project_id>")]
pub fn show_project_id(project_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    let project = get_project_by_id_pooled_safe(project_id);
    
    let ctx = json!({
        "project": project,
        "user": user
    });
    
    Ok(Template::render("project_detail", ctx))
}

#[get("/new_project")]
pub fn new_project(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if user is admin
    if !user.is_admin {
        let context = json!({
            "user": user,
            "title": "Access Denied"
        });
        return Ok(Template::render("access_denied", context));
    }
    
    let users = get_users_all().unwrap_or_default();
    
    let ctx = json!({
        "users": users,
        "user": user
    });
    Ok(Template::render("new_project", ctx))
}

#[post("/new_project", data = "<new_project>")]
pub fn post_project(new_project: Form<NewProject>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if user is admin
    if !user.is_admin {
        return Err(Redirect::to(uri!(show_projects)));
    }
    
    let connection = &mut get_db_connection().map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(new_project))
    })?;
    
    let project_data = new_project.into_inner();
    let result = insert_new_project(connection, &project_data);
    match result {
        Ok(project_id) => {
            // Log the project creation
            if let Ok(new_values) = Logger::to_json_string(&project_data) {
                let _ = Logger::log_create(
                    connection,
                    user.user_id,
                    EntityType::Project,
                    project_id,
                    None,
                    Some(new_values),
                    Some(format!("Created project: {}", project_data.project_name)),
                    None,
                );
            }
            
            // Invalidate cache for the new project
            invalidate_project_cache_complete(project_id);
            
            Ok(Redirect::to(uri!(show_projects)))
        },
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(new_project)))
        }
    }
}

#[get("/edit_project/<project_id>")]
pub fn get_edit_project(project_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if user is admin
    if !user.is_admin {
        let context = json!({
            "user": user,
            "title": "Access Denied"
        });
        return Ok(Template::render("access_denied", context));
    }
    
    let project = get_project_by_id_pooled_safe(project_id);
    let users = get_users_all_cached().unwrap_or_default();
    
    let ctx = json!({
        "project": project,
        "users": users,
        "user": user
    });
    Ok(Template::render("edit_project", ctx))
}

#[post("/edit_project/<project_id>", data = "<project>")]
pub fn post_edit_project(project_id: i32, project: Form<UpdateProject>, cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if user is admin
    if !user.is_admin {
        return Err(Redirect::to(uri!(show_projects)));
    }
    
    let connection = &mut get_db_connection().map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(get_edit_project(project_id)))
    })?;
    
    // Get the old values before updating
    let old_project = get_project_by_id(project_id);
    
    let result = edit_project(connection, project_id, &project);
    match result {
        Ok(_) => {
            // Log the project update
            let project_data = project.into_inner();
            if let (Ok(old_values), Ok(new_values)) = (Logger::to_json_string(&old_project), Logger::to_json_string(&project_data)) {
                let _ = Logger::log_update(
                    connection,
                    user.user_id,
                    EntityType::Project,
                    project_id,
                    None,
                    Some(old_values),
                    Some(new_values),
                    Some(format!("Updated project: {}", project_data.project_name)),
                    None,
                );
            }
            
            // Invalidate cache for the updated project
            invalidate_project_cache_complete(project_id);
            
            Ok(Redirect::to(uri!(show_projects)))
        },
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(Redirect::to(uri!(get_edit_project(project_id))))
        }
    }
}

#[delete("/delete_project/<project_id>")]
pub fn delete_project_route(project_id: i32, cookies: &CookieJar<'_>) -> Result<rocket::http::Status, Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if user is admin
    if !user.is_admin {
        return Err(Redirect::to(uri!(show_projects)));
    }
    
    let mut connection = match get_db_connection() {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(Redirect::to(uri!(show_projects)));
        }
    };
    
    // Get the project details before deleting
    let project = get_project_by_id_pooled_safe(project_id);
    
    let result = delete_project(connection.as_mut(), &project_id);
    match result {
        Ok(_) => {
            // Log the project deletion
            if let Ok(old_values) = Logger::to_json_string(&project) {
                let _ = Logger::log_delete(
                    connection.as_mut(),
                    user.user_id,
                    EntityType::Project,
                    project_id,
                    None,
                    Some(old_values),
                    Some(format!("Deleted project: {}", project.project_name)),
                    None,
                );
            }
            
            // Invalidate cache for the deleted project
            invalidate_project_cache_complete(project_id);
            
            Ok(rocket::http::Status::Ok)
        },
        Err(_e) => {
            #[cfg(debug_assertions)]
            println!("Error.*: {:?}", _e);
            Ok(rocket::http::Status::InternalServerError)
        }
    }
}

// Excel Import Routes
#[get("/import_excel")]
pub fn import_excel_page(cookies: &CookieJar<'_>) -> Result<content::RawHtml<String>, Redirect> {
    let _user = require_auth(cookies)?;
    
    // Get selected project ID and name
    let selected_project_id = get_selected_project_id(cookies);
    let (project_id, project_name) = if let Some(pid) = selected_project_id {
        let project = get_project_by_id_pooled_safe(pid);
        (pid, project.project_name)
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all_cached().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            (first_project.project_id, first_project.project_name.clone())
        } else {
            (1, "Default Project".to_string())
        }
    };
    
    let html = format!(r#"
    <!doctype html>
    <html lang='en'>
    <head>
        <title>ReqMan - Import Excel</title>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
        <link rel='stylesheet' href='/static/reqman.css'>
    </head>
    <body>
        <div class="container mt-4">
            <div class="row">
                <div class="col-md-8 offset-md-2">
                    <div class="card">
                        <div class="card-header">
                            <h3>Import Excel File</h3>
                        </div>
                        <div class="card-body">
                            <div class="alert alert-info">
                                <strong>Target Project:</strong> {} (ID: {})
                                <br>
                                <small class="text-muted">Requirements and tests will be imported into this project. You can change the project using the dropdown in the navigation bar above.</small>
                            </div>
                            <p>Upload an Excel file to import requirements or tests into the selected project.</p>
                            <form action="/import_excel/upload" method="post" enctype="multipart/form-data">
                                <div class="mb-3">
                                    <label for="excel_file" class="form-label">Select Excel File</label>
                                    <input type="file" class="form-control" id="excel_file" name="file" accept=".xlsx,.xls" required>
                                    <div class="form-text">Supported formats: .xlsx, .xls</div>
                                </div>
                                <div class="mt-3">
                                    <button type="submit" class="btn btn-primary">Upload File</button>
                                    <a href="/" class="btn btn-secondary">Cancel</a>
                                </div>
                            </form>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </body>
    </html>
    "#, project_name, project_id);
    
    Ok(content::RawHtml(html))
}

#[post("/import_excel/upload", data = "<upload>")]
pub async fn upload_excel_file(
    mut upload: rocket::form::Form<rocket::fs::TempFile<'_>>,
    cookies: &CookieJar<'_>,
) -> Result<content::RawHtml<String>, Redirect> {
    let _user = require_auth(cookies)?;
    
    // Save uploaded file temporarily
    let temp_path = format!("/tmp/upload_{}.xlsx", chrono::Utc::now().timestamp());
    upload.persist_to(&temp_path).await.map_err(|_| Redirect::to(uri!(import_excel_page)))?;
    
    // Parse Excel file
    let importer = crate::importers::excel::ExcelImporter::new(&temp_path).map_err(|_| Redirect::to(uri!(import_excel_page)))?;
    
    // Create HTML for column mapping
    let _columns_html = importer.columns.iter()
        .map(|col| format!("<option value=\"{}\">{}</option>", col.name, col.name))
        .collect::<Vec<_>>()
        .join("");
    
    let available_fields_html = importer.get_available_fields().iter()
        .map(|field| format!("<option value=\"{}\">{}</option>", field, field))
        .collect::<Vec<_>>()
        .join("");
    
    let html = format!(r#"
    <!doctype html>
    <html lang='en'>
    <head>
        <title>ReqMan - Map Excel Columns</title>
        <meta charset="utf-8">
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
        <link rel='stylesheet' href='/static/reqman.css'>
    </head>
    <body>
        <div class="container mt-4">
            <div class="row">
                <div class="col-md-10 offset-md-1">
                    <div class="card">
                        <div class="card-header">
                            <h3>Map Excel Columns</h3>
                            <p class="mb-0">Import Type: <strong>{}</strong> | Data Rows: <strong>{}</strong></p>
                        </div>
                        <div class="card-body">
                            <form action="/import_excel/process" method="post" id="mapping-form">
                                <input type="hidden" name="import_type" value="{}">
                                <input type="hidden" name="temp_file" value="{}">
                                <input type="hidden" name="column_mappings" id="column_mappings" value="">
                                
                                <div class="table-responsive">
                                    <table class="table table-bordered">
                                        <thead>
                                            <tr>
                                                <th>Excel Column</th>
                                                <th>Map To Field</th>
                                                <th>Sample Data</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {}
                                        </tbody>
                                    </table>
                                </div>
                                
                                <div class="mt-3">
                                    <button type="submit" class="btn btn-primary">Import Data</button>
                                    <a href="/import_excel" class="btn btn-secondary">Cancel</a>
                                </div>
                            </form>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        
        <script>
        document.addEventListener('DOMContentLoaded', function() {{
            const form = document.getElementById('mapping-form');
            form.addEventListener('submit', function(e) {{
                e.preventDefault();
                
                const mappings = [];
                const rows = document.querySelectorAll('tbody tr');
                rows.forEach(function(row) {{
                    const column = row.querySelector('td:first-child').textContent.trim();
                    const field = row.querySelector('select[name^="field"]').value;
                    if (field && field !== '') {{
                        mappings.push({{
                            excel_column: column,
                            target_field: field
                        }});
                    }}
                }});
                
                document.getElementById('column_mappings').value = JSON.stringify(mappings);
                form.submit();
            }});
        }});
        </script>
    </body>
    </html>
    "#,
    importer.import_type,
    importer.data.len(),
    importer.import_type,
    temp_path,
    importer.columns.iter()
        .map(|col| {
            let sample_data = &col.sample_value;
            format!(
                r#"<tr>
                    <td>{}</td>
                    <td>
                        <select name="field_{}" class="form-select">
                            <option value="">-- Select Field --</option>
                            {}
                        </select>
                    </td>
                    <td><small class="text-muted">{}</small></td>
                </tr>"#,
                col.name,
                col.name.replace(" ", "_"),
                available_fields_html,
                sample_data
            )
        })
        .collect::<Vec<_>>()
        .join("")
    );
    
    Ok(content::RawHtml(html))
}

#[post("/import_excel/process", data = "<mapping_data>")]
pub fn process_excel_import(
    mapping_data: Form<crate::models::ImportMappingForm>,
    cookies: &CookieJar<'_>,
) -> Result<content::RawHtml<String>, Redirect> {
    let _user = require_auth(cookies)?;
    
    eprintln!("Column mappings string: {}", mapping_data.column_mappings);
    
    // Parse column mappings
    let column_mappings: Vec<crate::importers::excel::ColumnMapping> = serde_json::from_str(&mapping_data.column_mappings)
        .map_err(|e| {
            eprintln!("JSON parsing error: {}", e);
            Redirect::to(uri!(import_excel_page))
        })?;
    
    // Create importer and import data
    let importer = crate::importers::excel::ExcelImporter::new(&mapping_data.temp_file)
        .map_err(|e| {
            eprintln!("Excel importer creation error: {}", e);
            Redirect::to(uri!(import_excel_page))
        })?;
    
    // Get selected project ID from cookies
    let selected_project_id = get_selected_project_id(cookies);
    let project_id = if let Some(pid) = selected_project_id {
        pid
    } else {
        // Default to the first project if no project is selected
        let projects = get_projects_all_cached().unwrap_or_default();
        if let Some(first_project) = projects.first() {
            first_project.project_id
        } else {
            1 // Fallback to project 1 if no projects exist
        }
    };
    
    // Create import configuration
    let config = crate::importers::excel::ImportConfig {
        import_type: mapping_data.import_type.clone(),
        column_mappings,
        project_id,
    };
    
    let connection = &mut get_db_connection().map_err(|e| {
        eprintln!("Database connection error: {}", e);
        Redirect::to(uri!(import_excel_page))
    })?;
    let result = importer.import_data(&config, connection);
    
    eprintln!("Import result: {:?}", result);
    
    let html = match result {
        Ok(import_result) => {
            // Invalidate all caches after successful import since we don't know exactly what was imported
            crate::cache::invalidate_all_cache();
            
            // Get project name for display
            let project_name = get_project_by_id_pooled_safe(project_id).project_name;
            
            format!(r#"
            <!doctype html>
            <html lang='en'>
            <head>
                <title>ReqMan - Import Results</title>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
                <link rel='stylesheet' href='/static/reqman.css'>
            </head>
            <body>
                <div class="container mt-4">
                    <div class="row">
                        <div class="col-md-8 offset-md-2">
                            <div class="card border-success">
                                <div class="card-header bg-success text-white">
                                    <h3><i class="fas fa-check-circle"></i> Import Successful</h3>
                                </div>
                                <div class="card-body">
                                    <div class="alert alert-success">
                                        <h5>Import completed successfully!</h5>
                                        <p><strong>Records imported:</strong> {}</p>
                                        <p><strong>Import type:</strong> {}</p>
                                        <p><strong>Target project:</strong> {} (ID: {})</p>
                                    </div>
                                    
                                    <div class="mt-3">
                                        <a href="/" class="btn btn-primary">Back to Home</a>
                                        <a href="/import_excel" class="btn btn-outline-primary">Import Another File</a>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </body>
            </html>
            "#,
            import_result.imported_count,
            mapping_data.import_type,
            project_name,
            project_id
            )
        }
        Err(e) => {
            // Get project name for display
            let project_name = get_project_by_id_pooled_safe(project_id).project_name;
            
            format!(r#"
            <!doctype html>
            <html lang='en'>
            <head>
                <title>ReqMan - Import Error</title>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <link href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.0/dist/css/bootstrap.min.css" rel="stylesheet">
                <link rel='stylesheet' href='/static/reqman.css'>
            </head>
            <body>
                <div class="container mt-4">
                    <div class="row">
                        <div class="col-md-8 offset-md-2">
                            <div class="card border-danger">
                                <div class="card-header bg-danger text-white">
                                    <h3><i class="fas fa-exclamation-triangle"></i> Import Failed</h3>
                                </div>
                                <div class="card-body">
                                    <div class="alert alert-danger">
                                        <h5>Import failed!</h5>
                                        <p><strong>Error:</strong> {}</p>
                                        <p><strong>Target project:</strong> {} (ID: {})</p>
                                    </div>
                                    
                                    <div class="mt-3">
                                        <a href="/import_excel" class="btn btn-primary">Try Again</a>
                                        <a href="/" class="btn btn-secondary">Back to Home</a>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </body>
            </html>
            "#,
            e,
            project_name,
            project_id
            )
        }
    };
    
    Ok(content::RawHtml(html))
}

// Admin Dashboard Routes
#[get("/admin")]
pub fn admin_dashboard(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if user is admin
    if !user.is_admin {
        let context = json!({
            "user": user,
            "title": "Access Denied"
        });
        return Ok(Template::render("access_denied", context));
    }
    
    let context = json!({
        "user": user,
        "title": "Admin Dashboard"
    });
    
    Ok(Template::render("admin/dashboard", context))
}

#[get("/admin/users")]
pub fn admin_users_page(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if user is admin
    if !user.is_admin {
        let context = json!({
            "user": user,
            "title": "Access Denied"
        });
        return Ok(Template::render("access_denied", context));
    }
    
    let users = get_users_all().unwrap_or_default();
    
    let context = json!({
        "user": user,
        "users": users,
        "title": "User Management"
    });
    
    Ok(Template::render("admin/users", context))
}

// Backup Routes
#[get("/admin/backup")]
pub fn admin_backup_page(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if user is admin
    if !user.is_admin {
        let context = json!({
            "user": user,
            "title": "Access Denied"
        });
        return Ok(Template::render("access_denied", context));
    }
    
    let context = json!({
        "user": user,
        "title": "Database Backup"
    });
    
    Ok(Template::render("admin/backup", context))
}

#[post("/admin/backup/generate/<filename>")]
pub async fn generate_backup(filename: String, cookies: &CookieJar<'_>) -> Result<(ContentType, NamedFile), Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if user is admin
    if !user.is_admin {
        return Err(Redirect::to(uri!(admin_backup_page)));
    }
    
    // Use the filename from the URL parameter
    let filename = if filename.ends_with(".sql") {
        filename
    } else {
        format!("{}.sql", filename)
    };
    
    // Create backup directory if it doesn't exist
    let backup_dir = "backups";
    if !std::path::Path::new(backup_dir).exists() {
        std::fs::create_dir(backup_dir).map_err(|_| Redirect::to(uri!(admin_backup_page)))?;
    }
    
    let backup_path = format!("{}/{}", backup_dir, filename);
    
    // Database configuration from Rocket.toml
    let _db_url = "postgres://rust:rust@127.0.0.1:5432/rust";
    let password = "rust";
    let host = "127.0.0.1";
    let port = "5432";
    let username = "rust";
    let database = "rust";
    
    // Set environment variable for password
    std::env::set_var("PGPASSWORD", password);
    
    // Execute pg_dump command with explicit table inclusion to ensure logs are included
    let output = std::process::Command::new("pg_dump")
        .args(&[
            "-h", host,
            "-p", port,
            "-U", username,
            "-d", database,
            "-f", &backup_path,
            "--no-password",
            "--verbose",  // Add verbose output for debugging
            "--no-owner",  // Don't include ownership information
            "--no-privileges"  // Don't include privilege information
        ])
        .output();
    
    match output {
        Ok(output) => {
            if output.status.success() {
                // Log the successful backup
                if let Ok(mut conn) = get_pooled_connection() {
                    let _ = Logger::log_action(
                        &mut conn,
                        user.user_id,
                        crate::models::ActionType::StatusChange,
                        crate::models::EntityType::User,
                        None,
                        None,
                        None,
                        None,
                        Some(format!("Database backup generated: {}", filename)),
                        None,
                    );
                }
                
                // Return the backup file for download
                let file = NamedFile::open(&backup_path)
                    .await
                    .map_err(|_| Redirect::to(uri!(admin_backup_page)))?;
                
                let content_type = ContentType::new("application", "sql");
                Ok((content_type, file))
            } else {
                // Log the failed backup
                if let Ok(mut conn) = get_pooled_connection() {
                    let _ = Logger::log_action(
                        &mut conn,
                        user.user_id,
                        crate::models::ActionType::StatusChange,
                        crate::models::EntityType::User,
                        None,
                        None,
                        None,
                        None,
                        Some(format!("Database backup failed: {}", String::from_utf8_lossy(&output.stderr))),
                        None,
                    );
                }
                
                // If backup failed, redirect to backup page with error
                Err(Redirect::to(uri!(admin_backup_page)))
            }
        }
        Err(e) => {
            // Log the command failure
            if let Ok(mut conn) = get_pooled_connection() {
                let _ = Logger::log_action(
                    &mut conn,
                    user.user_id,
                    crate::models::ActionType::StatusChange,
                    crate::models::EntityType::User,
                    None,
                    None,
                    None,
                    None,
                    Some(format!("Database backup command failed: {}", e)),
                    None,
                );
            }
            
            // If command failed, redirect to backup page with error
            Err(Redirect::to(uri!(admin_backup_page)))
        }
    }
}

#[get("/logs")]
pub fn show_logs(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if user is admin
    if !user.is_admin {
        let context = json!({
            "user": user,
            "title": "Access Denied"
        });
        return Ok(Template::render("access_denied", context));
    }
    
    let connection = &mut get_pooled_connection().map_err(|e| {
        eprintln!("Database connection error in show_logs: {}", e);
        Redirect::to(uri!(admin_dashboard))
    })?;
    let logs = Logger::get_recent_logs(connection, 1000).unwrap_or_default();
    
    // Enhance logs with user information
    let mut enhanced_logs = Vec::new();
    for log in logs {
        let username = get_user_by_id(log.user_id).user_username;
        let mut log_json = serde_json::to_value(log).unwrap_or_default();
        if let Some(log_obj) = log_json.as_object_mut() {
            log_obj.insert("username".to_string(), serde_json::Value::String(username));
        }
        enhanced_logs.push(log_json);
    }
    
    let ctx = json!({
        "user": user,
        "logs": enhanced_logs,
        "title": "System Logs"
    });
    
    Ok(Template::render("logs", ctx))
}

#[get("/logs/<entity_type>/<entity_id>")]
pub fn show_entity_logs(entity_type: String, entity_id: i32, cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if user is admin
    if !user.is_admin {
        let context = json!({
            "user": user,
            "title": "Access Denied"
        });
        return Ok(Template::render("access_denied", context));
    }
    
    let connection = &mut get_pooled_connection().map_err(|e| {
        eprintln!("Database connection error in show_entity_logs: {}", e);
        Redirect::to(uri!(show_logs))
    })?;
    let logs = Logger::get_logs_for_entity(connection, &entity_type, entity_id).unwrap_or_default();
    
    // Enhance logs with user information
    let mut enhanced_logs = Vec::new();
    for log in logs {
        let username = get_user_by_id(log.user_id).user_username;
        let mut log_json = serde_json::to_value(log).unwrap_or_default();
        if let Some(log_obj) = log_json.as_object_mut() {
            log_obj.insert("username".to_string(), serde_json::Value::String(username));
        }
        enhanced_logs.push(log_json);
    }
    
    let ctx = json!({
        "user": user,
        "logs": enhanced_logs,
        "entity_type": entity_type,
        "entity_id": entity_id,
        "title": format!("Logs for {} {}", entity_type, entity_id)
    });
    
    Ok(Template::render("entity_logs", ctx))
}

#[get("/export_logs?<filename>")]
pub async fn export_logs(filename: Option<String>, cookies: &CookieJar<'_>) -> Result<(ContentType, NamedFile), Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if user is admin
    if !user.is_admin {
        return Err(Redirect::to(uri!(show_logs)));
    }
    
    let connection = &mut get_pooled_connection().map_err(|e| {
        eprintln!("Database connection error in export_logs: {}", e);
        Redirect::to(uri!(show_logs))
    })?;
    let logs = Logger::get_recent_logs(connection, 1000).unwrap_or_default();
    
    // Convert logs to JSON
    let logs_json = serde_json::to_string_pretty(&logs).unwrap_or_default();
    
    // Generate filename if not provided
    let filename = filename.unwrap_or_else(|| {
        let now = chrono::Utc::now();
        format!("reqman-logs_{}.json", now.format("%Y%m%d_%H%M%S"))
    });
    
    // Ensure filename has .json extension
    let filename = if filename.ends_with(".json") {
        filename
    } else {
        format!("{}.json", filename)
    };
    
    // Create exports directory if it doesn't exist
    let export_dir = "exports";
    if !std::path::Path::new(export_dir).exists() {
        std::fs::create_dir(export_dir).map_err(|_| Redirect::to(uri!(show_logs)))?;
    }
    
    let export_path = format!("{}/{}", export_dir, filename);
    
    // Write JSON to file
    std::fs::write(&export_path, logs_json)
        .map_err(|_| Redirect::to(uri!(show_logs)))?;
    
    // Log the successful export
    let _ = Logger::log_export(
        connection,
        user.user_id,
        crate::models::EntityType::User,
        None,
        None,
        Some(format!("Exported logs to {}", filename)),
        None,
    );
    
    Ok((ContentType::JSON, NamedFile::open(export_path).await.map_err(|_| Redirect::to(uri!(show_logs)))?))
}

#[get("/export_logs/<entity_type>/<entity_id>")]
pub fn export_entity_logs(entity_type: String, entity_id: i32, cookies: &CookieJar<'_>) -> Result<(rocket::http::ContentType, String), Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if user is admin
    if !user.is_admin {
        return Err(Redirect::to(uri!(show_logs)));
    }
    
    let mut connection = match get_connection_pooled_safe() {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(Redirect::to(uri!(show_logs)));
        }
    };
    let logs = Logger::get_logs_for_entity(connection.as_mut(), &entity_type, entity_id).unwrap_or_default();
    
    // Convert logs to JSON
    let logs_json = serde_json::to_string_pretty(&logs).unwrap_or_default();
    
    let content_type = rocket::http::ContentType::new("application", "json");
    Ok((content_type, logs_json))
}

#[post("/cleanup_logs")]
pub fn cleanup_logs(cookies: &CookieJar<'_>) -> Result<Redirect, Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if user is admin
    if !user.is_admin {
        return Err(Redirect::to(uri!(show_logs)));
    }
    
    let mut connection = match get_connection_pooled_safe() {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(Redirect::to(uri!(show_logs)));
        }
    };
    
    // Clean up logs older than 90 days
    match crate::logger::cleanup_old_logs(connection.as_mut(), 90) {
        Ok(deleted_count) => {
            // Log the cleanup action
            let _ = Logger::log_action(
                connection.as_mut(),
                user.user_id,
                crate::models::ActionType::StatusChange,
                crate::models::EntityType::User,
                None,
                None,
                None,
                None,
                Some(format!("Cleaned up {} old log entries", deleted_count)),
                None,
            );
        },
        Err(_) => {
            // Log the failed cleanup action
            let _ = Logger::log_action(
                connection.as_mut(),
                user.user_id,
                crate::models::ActionType::StatusChange,
                crate::models::EntityType::User,
                None,
                None,
                None,
                None,
                Some("Failed to clean up old log entries".to_string()),
                None,
            );
        }
    }
    
    Ok(Redirect::to(uri!(show_logs)))
}

#[get("/log_analytics")]
pub fn log_analytics(cookies: &CookieJar<'_>) -> Result<Template, Redirect> {
    let user = require_auth(cookies)?;
    
    // Check if user is admin
    if !user.is_admin {
        let context = json!({
            "user": user,
            "title": "Access Denied"
        });
        return Ok(Template::render("access_denied", context));
    }
    
    let mut connection = match get_connection_pooled_safe() {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("Database connection error: {}", e);
            return Err(Redirect::to(uri!(show_logs)));
        }
    };
    
    // Get basic statistics
    let last_7_days = Logger::get_log_count(connection.as_mut(), 7).unwrap_or(0);
    let last_30_days = Logger::get_log_count(connection.as_mut(), 30).unwrap_or(0);
    let last_90_days = Logger::get_log_count(connection.as_mut(), 90).unwrap_or(0);
    
    let ctx = json!({
        "user": user,
        "last_7_days": last_7_days,
        "last_30_days": last_30_days,
        "last_90_days": last_90_days,
        "title": "Log Analytics"
    });
    
    Ok(Template::render("log_analytics", ctx))
}



// Test route for PDF generation (no authentication required)
#[get("/test-pdf")]
pub fn test_pdf_generation() -> Result<(rocket::http::ContentType, Vec<u8>), rocket::http::Status> {
    // Test data
    let total_requirements = 150;
    let total_tests = 120;
    let total_categories = 8;
    let total_users = 12;
    let coverage_percentage = 85.5;
    let avg_tests_per_requirement = 1.2;
    let covered_requirements = 128;
    let total_links = 180;
    
    let mut requirements_by_status = std::collections::HashMap::new();
    requirements_by_status.insert("Active".to_string(), 100);
    requirements_by_status.insert("Draft".to_string(), 30);
    requirements_by_status.insert("Deprecated".to_string(), 20);
    
    let mut tests_by_status = std::collections::HashMap::new();
    tests_by_status.insert("Passed".to_string(), 80);
    tests_by_status.insert("Failed".to_string(), 15);
    tests_by_status.insert("Pending".to_string(), 25);
    
    let mut requirements_by_category = std::collections::HashMap::new();
    requirements_by_category.insert("Functional".to_string(), 80);
    requirements_by_category.insert("Non-Functional".to_string(), 40);
    requirements_by_category.insert("Interface".to_string(), 30);
    
    // Generate PDF using the PDF generation function
    match generate_pdf_report_data(
        total_requirements,
        total_tests,
        total_categories,
        total_users,
        coverage_percentage,
        avg_tests_per_requirement,
        covered_requirements,
        total_links,
        requirements_by_status,
        tests_by_status,
        requirements_by_category
    ) {
        Ok(pdf_bytes) => {
            let content_type = rocket::http::ContentType::new("application", "pdf");
            Ok((content_type, pdf_bytes))
        }
        Err(e) => {
            eprintln!("PDF generation failed: {:?}", e);
            Err(rocket::http::Status::InternalServerError)
        }
    }
}

