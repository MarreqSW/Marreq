use std::collections::HashMap;

use rocket::form::{Form, FromForm};
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket::serde::json::{json, serde_json, Json};
use rocket::serde::Deserialize;
use rocket::State;
use rocket_dyn_templates::Template;

use super::prelude::*;

use crate::app::AppState;
use crate::helper_functions::{decorate_tests_with_repo, generate_requirement_reference};
use crate::models::*;
use crate::repository::errors::RepoError;
use crate::services::{
    ApplicabilityService, CategoryService, LogService, ProjectService, RequirementService,
    StatusService, UserService, VerificationService,
};

#[derive(FromForm)]
struct RequirementCreateForm {
    #[field(name = uncased("intent"))]
    intent: Option<String>,
    #[field(name = uncased("req_id"))]
    req_id: Option<i32>,
    #[field(name = uncased("req_title"))]
    req_title: String,
    #[field(name = uncased("req_description"))]
    req_description: String,
    #[field(name = uncased("req_verification"))]
    req_verification: i32,
    #[field(name = uncased("req_author"))]
    req_author: i32,
    #[field(name = uncased("req_link"))]
    req_link: String,
    #[field(name = uncased("req_category"))]
    req_category: i32,
    #[field(name = uncased("req_current_status"))]
    req_current_status: i32,
    #[field(name = uncased("req_parent"))]
    req_parent: i32,
    #[field(name = uncased("req_reference"))]
    req_reference: String,
    #[field(name = uncased("req_reviewer"))]
    req_reviewer: i32,
    #[field(name = uncased("req_applicability"))]
    req_applicability: i32,
    #[field(name = uncased("req_justification"))]
    req_justification: Option<String>,
    #[field(name = uncased("project_id"))]
    project_id: i32,
    #[field(name = uncased("req_purpose"))]
    req_purpose: Option<String>,
}

impl RequirementCreateForm {
    fn into_payload(self) -> (NewRequirement, Option<String>) {
        let RequirementCreateForm {
            intent,
            req_id,
            req_description,
            req_purpose,
            req_verification,
            req_author,
            req_link,
            req_category,
            req_current_status,
            req_parent,
            req_reference,
            req_reviewer,
            req_applicability,
            req_justification,
            project_id,
            req_title,
        } = self;

        let mut composed_description = req_description.trim().to_string();
        if let Some(purpose_raw) = req_purpose {
            let purpose = purpose_raw.trim();
            if !purpose.is_empty() {
                if composed_description.is_empty() {
                    composed_description = purpose.to_string();
                } else {
                    composed_description = format!("{purpose}\n\n{composed_description}");
                }
            }
        }

        let requirement = NewRequirement {
            req_id,
            req_title,
            req_description: composed_description,
            req_verification,
            req_author,
            req_link,
            req_category,
            req_current_status,
            req_parent,
            req_reference,
            req_reviewer,
            req_applicability,
            req_justification,
            project_id,
        };

        (requirement, intent)
    }
}

fn map_repo_error(err: RepoError) -> rocket::http::Status {
    match err {
        RepoError::BadInput(_) => rocket::http::Status::BadRequest,
        RepoError::NotFound => rocket::http::Status::NotFound,
        _ => rocket::http::Status::InternalServerError,
    }
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct InlineCategoryPayload {
    title: String,
    description: String,
    tag: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct InlineApplicabilityPayload {
    title: String,
    description: String,
    tag: String,
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct InlineVerificationPayload {
    name: String,
    description: String,
}

#[get("/<project_id>/requirements?<status_filter>&<verification_filter>&<category_filter>")]
async fn show_requirements(
    project_access: ProjectAccess,
    project_id: i32,
    cookies: &CookieJar<'_>,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    let project_service = ProjectService::new(state.inner());
    let selected_project = project_service.get_by_id(project_id).unwrap();

    cookies.add(Cookie::new("selected_project_id", project_id.to_string()));

    let requirement_service = RequirementService::new(state.inner());
    let filtered = requirement_service
        .list_by_project_filtered(
            project_id,
            status_filter,
            verification_filter,
            category_filter,
        )
        .unwrap_or_default();

    let decorated = {
        let repo = state.repo_read();
        decorate_requirements_with_repo(&*repo, filtered)
    };

    let total_requirements = decorated.len();
    let mut status_totals: HashMap<String, usize> = HashMap::new();

    for requirement in &decorated {
        let key = requirement.req_current_status.trim().to_ascii_lowercase();
        *status_totals.entry(key).or_default() += 1;
    }

    let draft_count = *status_totals.get("draft").unwrap_or(&0);
    let accepted_count = *status_totals.get("accepted").unwrap_or(&0);
    let rejected_count = *status_totals.get("rejected").unwrap_or(&0);

    let coverage_ratio = if total_requirements > 0 {
        accepted_count as f64 / total_requirements as f64
    } else {
        0.0
    };
    let coverage_percent = (coverage_ratio * 100.0).round() as i32;

    // Static lists; all default to empty on error
    let status_service = StatusService::new(state.inner());
    let statuses = status_service.list_legacy().unwrap_or_default();

    let category_service = CategoryService::new(state.inner());
    let categories = category_service
        .list_by_project(project_id)
        .unwrap_or_default();

    let verifications = {
        let repo = state.repo_read();
        repo.get_verification_by_project(project_id)
            .unwrap_or_default()
    };

    let status_lookup: HashMap<i32, String> = statuses
        .iter()
        .map(|s| (s.st_id, s.st_title.clone()))
        .collect();
    let verification_lookup: HashMap<i32, String> = verifications
        .iter()
        .map(|v| (v.verification_id, v.verification_name.clone()))
        .collect();
    let category_lookup: HashMap<i32, String> = categories
        .iter()
        .map(|c| (c.cat_id, c.cat_title.clone()))
        .collect();

    let status_label = status_filter.and_then(|id| status_lookup.get(&id).cloned());
    let verification_label =
        verification_filter.and_then(|id| verification_lookup.get(&id).cloned());
    let category_label = category_filter.and_then(|id| category_lookup.get(&id).cloned());

    let mut active_filters = Vec::new();
    if let Some(label) = status_label.clone() {
        active_filters.push(json!({
            "key": "status_filter",
            "label": format!("Status: {label}")
        }));
    }
    if let Some(label) = verification_label.clone() {
        active_filters.push(json!({
            "key": "verification_filter",
            "label": format!("Verification: {label}")
        }));
    }
    if let Some(label) = category_label.clone() {
        active_filters.push(json!({
            "key": "category_filter",
            "label": format!("Category: {label}")
        }));
    }

    let ctx = json!({
        "requirements": json!(decorated),
        "requirement_metrics": json!({
            "total": total_requirements,
            "draft": draft_count,
            "accepted": accepted_count,
            "rejected": rejected_count,
            "coverage": {
                "verified": accepted_count,
                "percent": coverage_percent
            }
        }),
       "selected_project_id": json!(project_id),

        // Filters for template state
        "statuses": json!(statuses),
        "verifications": json!(verifications),
        "categories": json!(categories),
        "current_status_filter": json!(status_filter),
        "current_verification_filter": json!(verification_filter),
        "current_category_filter": json!(category_filter),
        "active_filters": json!(active_filters),
        "project": json!({
            "id": selected_project.project_id,
            "name": selected_project.project_name,
            "status": selected_project.project_status,
            "description": selected_project.project_description,
        }),
        "user": user,
    });

    Ok(Template::render("requirements", ctx))
}

#[get("/<project_id>/requirements/show/<req_id>")]
async fn show_requirement_id(
    project_access: ProjectAccess,
    project_id: i32,
    req_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    let requirement_service = RequirementService::new(state.inner());
    let project_service = ProjectService::new(state.inner());
    let log_service = LogService::new(state.inner());

    let raw_requirement = match requirement_service.get_by_id(req_id) {
        Ok(req) => req,
        Err(crate::repository::errors::RepoError::NotFound) => {
            let ctx = json!({
                "title": "Requirement Not Found",
                "message": "The requirement you're looking for could not be found.",
                "details": "The specified requirement does not exist.",
                "user": user
            });
            return Ok(Template::render("error", ctx));
        }
        Err(err) => {
            let ctx = json!({
                "title": "Error Loading Requirement",
                "message": "An error occurred while loading the requirement.",
                "details": format!("{:?}", err),
                "user": user
            });
            return Ok(Template::render("error", ctx));
        }
    };

    // Enforce project ownership
    if raw_requirement.project_id != project_id {
        let reqs_url = uri!(
            "/p",
            show_requirements(
                project_id = raw_requirement.project_id,
                status_filter = Option::<i32>::None,
                verification_filter = Option::<i32>::None,
                category_filter = Option::<i32>::None
            )
        );

        eprintln!(
            "Project ID mismatch: route {}, requirement {}",
            project_id, raw_requirement.project_id
        );

        return Err(Redirect::to(reqs_url));
    }

    let project = project_service.get_by_id(project_id).ok();

    let decorated_requirement = {
        let repo = state.repo_read();
        let mut decorated = decorate_requirements_with_repo(&*repo, vec![raw_requirement.clone()]);
        decorated.pop()
    };

    let Some(requirement) = decorated_requirement else {
        let ctx = json!({
            "title": "Requirement Error",
            "message": "The requirement could not be displayed.",
            "details": "Failed to prepare requirement details for rendering.",
            "user": user
        });
        return Ok(Template::render("error", ctx));
    };

    // Relationship lookups
    let parent_requirement = if requirement.req_parent_id != 0 {
        match requirement_service.get_by_id(requirement.req_parent_id) {
            Ok(parent_raw) => {
                let repo = state.repo_read();
                let mut decorated =
                    decorate_requirements_with_repo(&*repo, vec![parent_raw.clone()]);
                decorated.pop()
            }
            Err(_) => None,
        }
    } else {
        None
    };

    let child_requirements = match requirement_service.list_by_project(project_id) {
        Ok(all) => {
            let children: Vec<Requirement> = all
                .into_iter()
                .filter(|r| r.req_parent == requirement.req_id)
                .collect();
            if children.is_empty() {
                Vec::new()
            } else {
                let repo = state.repo_read();
                decorate_requirements_with_repo(&*repo, children)
            }
        }
        Err(_) => Vec::new(),
    };

    // Linked verification artefacts
    let linked_tests_raw = requirement_service
        .get_linked_tests(req_id)
        .unwrap_or_default();
    let linked_tests = if linked_tests_raw.is_empty() {
        Vec::new()
    } else {
        let repo = state.repo_read();
        decorate_tests_with_repo(&*repo, linked_tests_raw)
    };

    let mut tests_passed = 0usize;
    let mut tests_failed = 0usize;
    let mut tests_pending = 0usize;

    for test in &linked_tests {
        match test.test_status.trim().to_ascii_lowercase().as_str() {
            "passed" => tests_passed += 1,
            "failed" => tests_failed += 1,
            _ => tests_pending += 1,
        }
    }

    let total_tests = linked_tests.len();
    let verification_percent = if total_tests > 0 {
        ((tests_passed as f64 / total_tests as f64) * 100.0).round() as i32
    } else {
        0
    };

    let verification_state = if total_tests == 0 {
        "No verifications linked yet"
    } else if tests_failed == 0 && tests_pending == 0 {
        "All linked verifications are passing"
    } else if tests_failed == 0 {
        "Verification in progress"
    } else {
        "Verification needs attention"
    };

    let verification_variant = if total_tests == 0 {
        "bg-warning"
    } else if tests_failed == 0 && tests_pending == 0 {
        "bg-primary"
    } else if tests_failed == 0 {
        "bg-info"
    } else {
        "bg-danger"
    };

    let status_variant = match requirement
        .req_current_status
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "accepted" | "finished" => "bg-success",
        "draft" | "proposal" => "bg-secondary",
        "rejected" | "cancelled" => "bg-danger",
        _ => "bg-secondary",
    };

    let solidity_label = if total_tests == 0 {
        if requirement
            .req_current_status
            .trim()
            .eq_ignore_ascii_case("draft")
        {
            "Needs definition"
        } else {
            "Unverified"
        }
    } else if tests_failed == 0 && tests_pending == 0 {
        "Rock solid"
    } else if tests_failed == 0 {
        "Under evaluation"
    } else {
        "At risk"
    };

    let solidity_variant = match solidity_label {
        "Rock solid" => "text-success",
        "Under evaluation" => "text-info",
        "At risk" => "text-danger",
        _ => "text-muted",
    };
    let solidity_description = match solidity_label {
        "Rock solid" => "All linked verifications have passed.",
        "Under evaluation" => "Waiting for pending verification results.",
        "At risk" => "At least one verification failed; needs attention.",
        _ => "No verification evidence linked yet.",
    };

    let reference = if requirement.req_reference.trim().is_empty() {
        format!("REQ-{:04}", requirement.req_id)
    } else {
        requirement.req_reference.clone()
    };

    let creation_date = raw_requirement
        .req_creation_date
        .format("%Y-%m-%d %H:%M")
        .to_string();
    let update_date = raw_requirement
        .req_update_date
        .format("%Y-%m-%d %H:%M")
        .to_string();
    let deadline_date = raw_requirement
        .req_deadline_date
        .format("%Y-%m-%d")
        .to_string();

    let author_initial = requirement
        .req_author
        .trim()
        .chars()
        .next()
        .map(|c| c.to_ascii_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string());
    let reviewer_initial = requirement
        .req_reviewer
        .trim()
        .chars()
        .next()
        .map(|c| c.to_ascii_uppercase().to_string());
    let reviewer_assigned = reviewer_initial.is_some();
    let reviewer_initial_value = reviewer_initial.clone();

    let purpose = requirement
        .req_description
        .split("\n\n")
        .next()
        .unwrap_or(&requirement.req_description)
        .trim()
        .to_string();
    let rationale = requirement
        .req_justification
        .clone()
        .unwrap_or_else(|| "No rationale documented yet.".to_string());

    let req_link = requirement.req_link.clone();
    let link_trimmed = req_link.trim().to_string();
    let notes = if link_trimmed.is_empty() {
        "No implementation notes recorded.".to_string()
    } else {
        format!("Primary reference available at {}", link_trimmed)
    };

    let attachments = if link_trimmed.is_empty() {
        Vec::new()
    } else {
        vec![json!({
            "label": "Supporting evidence",
            "href": req_link.clone(),
        })]
    };

    let comments_locked_reason = match requirement
        .req_current_status
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "accepted" => Some("Read-only: requirement accepted and locked".to_string()),
        "rejected" => Some("Archived requirement: comments are closed".to_string()),
        _ => None,
    };

    let comments_enabled = comments_locked_reason.is_none();
    let comment_items: Vec<serde_json::Value> = Vec::new();
    let comments_has_items = !comment_items.is_empty();

    let history_entries = log_service
        .entity_logs(&EntityType::Requirement.to_string(), req_id)
        .unwrap_or_default();
    let total_versions = history_entries.len().saturating_add(1);
    let mut timeline: Vec<serde_json::Value> = Vec::new();

    timeline.push(json!({
        "version": format!("v{}", total_versions),
        "summary": format!("Current revision — {}", requirement.req_current_status),
        "actor": if requirement.req_reviewer.trim().is_empty() {
            &requirement.req_author
        } else {
            &requirement.req_reviewer
        },
        "timestamp": update_date.clone(),
        "action": "CURRENT",
        "old_values": serde_json::Value::Null,
        "new_values": serde_json::Value::Null,
        "is_current": true
    }));

    for (index, entry) in history_entries.into_iter().enumerate() {
        let summary = entry
            .log
            .description
            .clone()
            .unwrap_or_else(|| format!("{} requirement", entry.log.action_type));
        let timestamp = entry.log.created_at.format("%Y-%m-%d %H:%M").to_string();
        timeline.push(json!({
            "version": format!("v{}", total_versions.saturating_sub(index + 1)),
            "summary": summary,
            "actor": entry.username,
            "timestamp": timestamp,
            "action": entry.log.action_type,
            "old_values": entry.log.old_values,
            "new_values": entry.log.new_values,
            "is_current": false
        }));
    }

    let current_version = timeline
        .first()
        .and_then(|item| item.get("version"))
        .and_then(|value| value.as_str())
        .unwrap_or("v1")
        .to_string();
    let reviewer_timestamp = if reviewer_assigned {
        Some(update_date.clone())
    } else {
        None
    };

    let ctx = json!({
        "user": user,
        "project": project.as_ref().map(|p| {
            json!({
                "id": p.project_id,
                "name": &p.project_name,
                "status": &p.project_status,
                "description": &p.project_description
            })
        }).unwrap_or(json!({ "id": project_id })),
        "selected_project_id": project_id,
        "requirement": &requirement,
        "reference": reference,
        "status_badge": {
            "label": &requirement.req_current_status,
            "variant": status_variant
        },
        "verification_badge": {
            "label": &requirement.req_verification,
            "variant": verification_variant,
            "state": verification_state
        },
        "solidity": {
            "label": solidity_label,
            "variant": solidity_variant,
            "description": solidity_description
        },
        "chips": [
            {
                "label": &requirement.req_category,
                "type": "category"
            },
            {
                "label": &requirement.req_applicability,
                "type": "applicability"
            }
        ],
        "metadata": {
            "author": {
                "name": &requirement.req_author,
                "timestamp": creation_date.clone(),
                "initial": author_initial
            },
            "reviewer": {
                "name": &requirement.req_reviewer,
                "timestamp": reviewer_timestamp,
                "initial": reviewer_initial_value,
                "assigned": reviewer_assigned
            },
            "updated": update_date.clone(),
            "deadline": deadline_date,
            "version": current_version
        },
        "body_sections": [
            {
                "title": "Purpose",
                "content": purpose
            },
            {
                "title": "Statement",
                "content": &requirement.req_description
            },
            {
                "title": "Rationale",
                "content": rationale
            },
            {
                "title": "Notes",
                "content": notes
            }
        ],
        "relationships": {
            "parent": parent_requirement,
            "children": child_requirements,
            "has_links": parent_requirement.is_some() || !child_requirements.is_empty()
        },
        "attachments": attachments,
        "verification_summary": {
            "total": total_tests,
            "passed": tests_passed,
            "failed": tests_failed,
            "pending": tests_pending,
            "percent": verification_percent,
            "last_checked": update_date.clone(),
            "tool": &requirement.req_verification
        },
        "linked_tests": linked_tests,
        "timeline": timeline,
        "comments": {
            "enabled": comments_enabled,
            "items": comment_items,
            "has_items": comments_has_items,
            "locked_reason": comments_locked_reason
        }
    });

    Ok(Template::render("requirement", ctx))
}

#[get("/<project_id>/requirements/edit/<req_id>")]
async fn get_edit_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    req_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let service = RequirementService::new(state.inner());

    let req = match service.get_by_id_decorated(req_id) {
        Ok(req) => req,
        Err(err) => {
            let ctx = json!({
                "title": "Error Loading Requirement",
                "message": "An error occurred while loading the requirement.",
                "details": format!("{:?}", err),
                "user": user
            });
            return Ok(Template::render("error", ctx));
        }
    };

    // Enforce project ownership; redirect if mismatched
    if req.project_id != project_id {
        eprintln!(
            "Project mismatch on edit: route {}, requirement {}",
            project_id, req.project_id
        );

        let url = uri!(
            "/p",
            show_requirements(
                project_id = req.project_id,
                status_filter = Option::<i32>::None,
                verification_filter = Option::<i32>::None,
                category_filter = Option::<i32>::None
            )
        );
        return Err(Redirect::to(url));
    }

    let parent: Option<Requirement> = if req.req_parent_id != 0 {
        match service.get_by_id(req.req_parent_id) {
            Ok(r) => Some(r),
            Err(_) => None,
        }
    } else {
        None
    };

    let project_service = ProjectService::new(state.inner());
    let project = project_service.get_by_id(project_id).ok();

    let log_service = LogService::new(state.inner());
    let history_entries = log_service
        .entity_logs(&EntityType::Requirement.to_string(), req_id)
        .unwrap_or_default();

    let version_counter = history_entries.len().saturating_add(1);
    let version_label = format!("v1.{}", version_counter.saturating_sub(1));
    let last_editor_name = history_entries
        .first()
        .map(|entry| entry.username.clone())
        .filter(|name| !name.is_empty())
        .or_else(|| {
            if !req.req_reviewer.trim().is_empty() {
                Some(req.req_reviewer.clone())
            } else if !req.req_author.trim().is_empty() {
                Some(req.req_author.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "Unknown author".to_string());

    let linked_candidates = service
        .list_by_project(project_id)
        .unwrap_or_default()
        .into_iter()
        .filter(|candidate| candidate.req_id != req_id)
        .map(|candidate| {
            json!({
                "id": candidate.req_id,
                "title": candidate.req_title,
                "reference": candidate.req_reference,
            })
        })
        .collect::<Vec<_>>();

    // Project-scoped lookups; default to empty on error
    let status_service = StatusService::new(state.inner());
    let statuses = status_service.list_legacy().unwrap_or_default();

    let category_service = CategoryService::new(state.inner());
    let categories = category_service
        .list_by_project(project_id)
        .unwrap_or_default();

    let user_service = UserService::new(state.inner());
    let users = user_service.list_all().unwrap_or_default();

    let verifications = {
        let repo = state.repo_read();
        repo.get_verification_by_project(project_id)
            .unwrap_or_default()
    };

    let applicability_service = ApplicabilityService::new(state.inner());
    let applicability = applicability_service
        .list_by_project(project_id)
        .unwrap_or_default();

    let display_reference = if req.req_reference.trim().is_empty() {
        format!("RM-{:03}", req.req_id)
    } else {
        req.req_reference.clone()
    };

    let ctx = json!({
        "req_author_id": req.req_author_id,
        "req_reviewer_id": req.req_reviewer_id,
        "req_category_id": req.req_category_id,
        "req_applicability_id": req.req_applicability_id,
        "req_current_status_id": req.req_current_status_id,
        "req_verification_id": req.req_verification_id,
        "req_parent_id": req.req_parent_id,
        "categories": categories,
        "status": statuses,
        "parent": parent,
        "users": users,
        "verification": verifications,
        "applicability": applicability,
        "user": user,
        "requirement": json!(req),
        "display_reference": display_reference,
        "project": project.map(|p| {
            json!({
                "id": p.project_id,
                "name": p.project_name,
                "status": p.project_status,
                "description": p.project_description,
            })
        }),
        "version": {
            "label": version_label,
            "last_editor": last_editor_name,
            "updated_at": req.req_update_date,
        },
        "linked_requirement_options": linked_candidates,
        "autosave": {
            "enabled": true,
            "interval_ms": 30_000
        }
    });

    #[cfg(debug_assertions)]
    println!("Edit requirement ctx: {:#}", ctx);

    Ok(Template::render("edit_requirement", ctx))
}

#[post("/<project_id>/requirements/edit/<req_id>", data = "<new_req>")]
async fn post_edit_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    req_id: i32,
    new_req: Form<NewRequirement>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();

    let edit_url = uri!("/p", get_edit_requirement(project_id, req_id));
    let list_url = uri!(
        "/p",
        show_requirements(
            project_id = project_id,
            status_filter = Option::<i32>::None,
            verification_filter = Option::<i32>::None,
            category_filter = Option::<i32>::None
        )
    );
    let show_url = uri!("/p", show_requirement_id(project_id, req_id));

    let service = RequirementService::new(state.inner());
    match service.update(&user, req_id, new_req.into_inner()) {
        Ok(_) => {}
        Err(crate::repository::errors::RepoError::NotFound) => return Err(Redirect::to(list_url)),
        Err(crate::repository::errors::RepoError::BadInput(_)) => {
            return Err(Redirect::to(edit_url))
        }
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!(
                "Error editing requirement {} in project {}: {:?}",
                req_id, project_id, _err
            );
            return Err(Redirect::to(list_url));
        }
    }

    Ok(Redirect::to(show_url))
}

#[delete("/<project_id>/requirements/delete/<req_id>")]
async fn delete_requirement_route(
    project_access: ProjectAccess,
    project_id: i32,
    req_id: i32,
    state: &State<AppState>,
) -> Result<Redirect, rocket::http::Status> {
    let user = project_access.into_user();
    let list_url = uri!(
        "/p",
        show_requirements(
            project_id = project_id,
            status_filter = Option::<i32>::None,
            verification_filter = Option::<i32>::None,
            category_filter = Option::<i32>::None
        )
    );

    // 1) Load requirement or 404
    let req = match RequirementService::new(state.inner()).get_by_id(req_id) {
        Ok(r) => r,
        Err(_) => return Err(rocket::http::Status::NotFound),
    };

    // 2) Enforce project ownership; if mismatched, just bounce to the right project’s list
    if req.project_id != project_id {
        let right_list = uri!(
            "/p",
            show_requirements(
                project_id = req.project_id,
                status_filter = Option::<i32>::None,
                verification_filter = Option::<i32>::None,
                category_filter = Option::<i32>::None
            )
        );
        return Ok(Redirect::to(right_list));
    }

    // 3) Permission gate: allow only Draft(1) or Proposal(2) or admin
    if req.req_current_status > 2 && !user.is_admin {
        return Err(rocket::http::Status::Forbidden);
    }

    // 4) Delete
    let service = RequirementService::new(state.inner());
    match service.delete(&user, req_id) {
        Ok(_) => {}
        Err(crate::repository::errors::RepoError::NotFound) => {
            return Err(rocket::http::Status::NotFound)
        }
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!("delete_requirement({}) failed: {:?}", req_id, _err);
            return Err(rocket::http::Status::InternalServerError);
        }
    }

    // 5) Redirect to this project’s requirements
    Ok(Redirect::to(list_url))
}

#[get("/<project_id>/requirements/new?<error>&<created>")]
async fn new_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    _cookies: &CookieJar<'_>,
    state: &State<AppState>,
    error: Option<String>,
    created: Option<String>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    let project_service = ProjectService::new(state.inner());
    let project = match project_service.get_by_id(project_id) {
        Ok(project) => project,
        Err(_) => {
            #[cfg(debug_assertions)]
            eprintln!(
                "Failed to load project context for new requirement page: project_id={}",
                project_id
            );
            return Err(Redirect::to(uri!(
                "/p",
                show_requirements(
                    project_id = project_id,
                    status_filter = Option::<i32>::None,
                    verification_filter = Option::<i32>::None,
                    category_filter = Option::<i32>::None
                )
            )));
        }
    };

    let parents = match RequirementService::new(state.inner()).list_by_project(project_id) {
        Ok(reqs) => reqs,
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!(
                "Failed to load parent requirements for project {}: {:?}",
                project_id, _err
            );
            Vec::new()
        }
    };

    let status_service = StatusService::new(state.inner());
    let statuses = status_service
        .list_requirement_statuses()
        .unwrap_or_default();

    let category_service = CategoryService::new(state.inner());
    let categories = category_service
        .list_by_project(project_id)
        .unwrap_or_default();

    let user_service = UserService::new(state.inner());
    let users = user_service.list_all().unwrap_or_default();

    let verifications = {
        let repo = state.repo_read();
        repo.get_verification_by_project(project_id)
            .unwrap_or_default()
    };

    let applicability_service = ApplicabilityService::new(state.inner());
    let applicability = applicability_service
        .list_by_project(project_id)
        .unwrap_or_default();

    let created_flash = created.and_then(|flag| {
        if flag == "1" || flag.eq_ignore_ascii_case("true") {
            Some("Requirement created successfully.".to_string())
        } else {
            None
        }
    });

    let created_timestamp = chrono::Utc::now()
        .naive_utc()
        .format("%Y-%m-%d")
        .to_string();

    let ctx = json!({
        "categories": categories,
        "status": statuses,
        "parent": parents,
        "users": users,
        "verification": verifications,
        "applicability": applicability,
        "project_id": project_id,
        "project": {
            "id": project.project_id,
            "name": project.project_name,
        },
        "selected_project_id": project_id,
        // empty defaults for the form
        "req_title": "",
        "req_description": "",
        "req_justification": "",
        "req_reference": "",
        "req_link": "",
        "req_purpose": "",
        "req_current_status": statuses
            .iter()
            .find(|st| st.req_st_title.eq_ignore_ascii_case("Draft"))
            .map(|st| st.req_st_id)
            .unwrap_or_else(|| statuses.first().map(|st| st.req_st_id).unwrap_or_default()),
        "created_timestamp": created_timestamp,
        "user": user,
        "error": error,
        "flash_success": created_flash,
    });

    Ok(Template::render("new_requirement", ctx))
}

#[post("/<project_id>/requirements/new", data = "<new_req>")]
async fn post_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    new_req: Form<RequirementCreateForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = project_access.into_user();

    // Reuse these URLs
    let new_url = uri!(
        "/p",
        new_requirement(
            project_id = project_id,
            error = Some("Invalid data provided".to_string()),
            created = Option::<String>::None
        )
    );

    // Take ownership and enforce project_id from the route
    let (mut req, intent) = new_req.into_inner().into_payload();
    req.project_id = project_id;

    // --- Reference validation / generation ---
    if !req.req_reference.is_empty() {
        // Validate against the category's tag
        let category = get_category_or_placeholder(state, req.req_category);
        let expected_prefix = format!("REQ-{}-", category.cat_tag);
        if !req.req_reference.starts_with(&expected_prefix) {
            return Err(Redirect::to(new_url));
        }

        // Strict pattern: REQ-<CAT_TAG>-<NUMBER>
        // Escape the tag just in case and compile once.
        let pat = format!(r"^REQ-{}-\d+$", regex::escape(&category.cat_tag));
        let re = match regex::Regex::new(&pat) {
            Ok(r) => r,
            Err(_e) => {
                #[cfg(debug_assertions)]
                eprintln!("regex compile failed for '{}': {:?}", pat, _e);
                return Err(Redirect::to(new_url));
            }
        };
        if !re.is_match(&req.req_reference) {
            return Err(Redirect::to(new_url));
        }
    } else {
        // Generate when missing
        let generated = {
            let repo = state.repo_read();
            generate_requirement_reference(&*repo, req.req_category, req.project_id)
        };

        match generated {
            Ok(reference) => req.req_reference = reference,
            Err(_e) => {
                #[cfg(debug_assertions)]
                eprintln!("reference generation failed: {:?}", _e);
                req.req_reference = format!("REQ-UNKNOWN-{}", chrono::Utc::now().timestamp());
            }
        }
    }

    let failure_url = uri!(
        "/p",
        new_requirement(
            project_id = project_id,
            error = Some("Failed to create requirement".to_string()),
            created = Option::<String>::None
        )
    );

    // --- Insert ---
    let service = RequirementService::new(state.inner());
    let req_id = match service.create(&user, req) {
        Ok(id) => id,
        Err(crate::repository::errors::RepoError::BadInput(_)) => {
            return Err(Redirect::to(new_url))
        }
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!("service create requirement failed: {:?}", _err);
            return Err(Redirect::to(failure_url));
        }
    };

    if matches!(intent.as_deref(), Some("add_another")) {
        return Ok(Redirect::to(uri!(
            "/p",
            new_requirement(
                project_id = project_id,
                error = Option::<String>::None,
                created = Some("1".to_string())
            )
        )));
    }

    // --- Success: show the new requirement ---
    Ok(Redirect::to(uri!(
        "/p",
        show_requirement_id(project_id, req_id)
    )))
}

#[get("/<project_id>/requirements/tree")]
async fn show_requirements_tree(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    // Only this project's requirements
    let reqs = match RequirementService::new(state.inner()).list_by_project(project_id) {
        Ok(reqs) => reqs,
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!(
                "Failed to load requirements for tree view (project {}): {:?}",
                project_id, _err
            );
            Vec::new()
        }
    };

    // Index children by parent_id; collect roots
    let mut children: HashMap<i32, Vec<&Requirement>> = HashMap::new();
    let mut roots: Vec<&Requirement> = Vec::new();

    for r in &reqs {
        if r.req_parent == 0 {
            roots.push(r);
        } else {
            children.entry(r.req_parent).or_default().push(r);
        }
    }

    // Sort roots and each child list by req_id for deterministic output
    roots.sort_by_key(|r| r.req_id);
    for v in children.values_mut() {
        v.sort_by_key(|r| r.req_id);
    }

    // Recursive builder
    fn build_node<'a>(
        req: &'a Requirement,
        idx: &HashMap<i32, Vec<&'a Requirement>>,
    ) -> serde_json::Value {
        let kids = idx
            .get(&req.req_id)
            .map(|vs| vs.iter().map(|c| build_node(c, idx)).collect::<Vec<_>>())
            .unwrap_or_default();

        json!({
            "requirement": req,
            "children": kids
        })
    }

    let tree = roots
        .into_iter()
        .map(|r| build_node(r, &children))
        .collect::<Vec<_>>();

    let ctx = json!({
        "tree_data": tree,
        "total_requirements": reqs.len(),
        "user": user,
        "project_id": project_id,
        "selected_project_id": project_id
    });

    Ok(Template::render("requirements_tree", ctx))
}

#[post(
    "/<project_id>/requirements/inline/category",
    format = "json",
    data = "<payload>"
)]
async fn create_category_inline(
    project_access: ProjectAccess,
    project_id: i32,
    payload: Json<InlineCategoryPayload>,
    state: &State<AppState>,
) -> Result<Json<serde_json::Value>, rocket::http::Status> {
    let user = project_access.into_user();
    let data = payload.into_inner();

    let category_service = CategoryService::new(state.inner());
    let new_category = NewCategory {
        cat_id: None,
        cat_title: data.title,
        cat_description: data.description,
        cat_tag: data.tag,
        project_id,
    };

    let id = category_service
        .create(&user, new_category)
        .map_err(map_repo_error)?;
    let stored = category_service.get_by_id(id).map_err(map_repo_error)?;

    Ok(Json(json!({
        "id": stored.cat_id,
        "label": stored.cat_title,
        "tag": stored.cat_tag,
    })))
}

#[post(
    "/<project_id>/requirements/inline/applicability",
    format = "json",
    data = "<payload>"
)]
async fn create_applicability_inline(
    project_access: ProjectAccess,
    project_id: i32,
    payload: Json<InlineApplicabilityPayload>,
    state: &State<AppState>,
) -> Result<Json<serde_json::Value>, rocket::http::Status> {
    let user = project_access.into_user();
    let data = payload.into_inner();

    let applicability_service = ApplicabilityService::new(state.inner());
    let new_applicability = NewApplicability {
        app_id: None,
        app_title: data.title,
        app_description: data.description,
        app_tag: data.tag,
        project_id,
    };

    let id = applicability_service
        .create(&user, new_applicability)
        .map_err(map_repo_error)?;
    let stored = applicability_service
        .get_by_id(id)
        .map_err(map_repo_error)?;

    Ok(Json(json!({
        "id": stored.app_id,
        "label": stored.app_title,
        "tag": stored.app_tag,
    })))
}

#[post(
    "/<project_id>/requirements/inline/verification",
    format = "json",
    data = "<payload>"
)]
async fn create_verification_inline(
    _project_access: ProjectAccess,
    project_id: i32,
    payload: Json<InlineVerificationPayload>,
    state: &State<AppState>,
) -> Result<Json<serde_json::Value>, rocket::http::Status> {
    let data = payload.into_inner();

    let verification_service = VerificationService::new(state.inner());
    let new_verification = NewVerification {
        verification_id: None,
        verification_name: data.name,
        verification_description: data.description,
        project_id,
    };

    let id = verification_service
        .create(new_verification)
        .map_err(map_repo_error)?;
    let stored = verification_service.get_by_id(id).map_err(map_repo_error)?;

    Ok(Json(json!({
        "id": stored.verification_id,
        "label": stored.verification_name,
        "description": stored.verification_description,
    })))
}

fn get_category_or_placeholder(state: &State<AppState>, category_id: i32) -> Category {
    CategoryService::new(state.inner())
        .get_by_id(category_id)
        .unwrap_or_else(|_| Category {
            cat_id: category_id,
            cat_title: format!("Unknown Category ({})", category_id),
            cat_description: "Category not found".to_string(),
            cat_tag: "unknown".to_string(),
            project_id: 1,
        })
}

pub fn routes() -> Vec<Route> {
    routes![
        show_requirements,
        show_requirement_id,
        get_edit_requirement,
        post_edit_requirement,
        delete_requirement_route,
        new_requirement,
        post_requirement,
        show_requirements_tree,
        create_category_inline,
        create_applicability_inline,
        create_verification_inline
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Project, ProjectMember};
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::routes::html::project::test_helpers::{
        client_with_routes, delete_with_session, get_with_session, post_form_with_session,
        session_cookie, timestamp, TestAppState,
    };
    use rocket::http::{ContentType, Cookie, Status};
    use rocket::local::asynchronous::Client;
    use rocket::serde::json::{serde_json, Value as JsonValue};

    const ADMIN_ID: i32 = 1;
    const PRIMARY_PROJECT: i32 = 1;

    fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();
        let mut admin = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
        admin.is_admin = true;
        repo.users.insert(ADMIN_ID, admin);

        // Add test data
        repo.projects.insert(
            PRIMARY_PROJECT,
            Project {
                project_id: PRIMARY_PROJECT,
                project_name: "Test Project".into(),
                project_description: Some("Description".into()),
                project_creation_date: Some(timestamp()),
                project_update_date: Some(timestamp()),
                project_status: Some("Active".into()),
                project_owner_id: Some(ADMIN_ID),
            },
        );

        // Add membership
        repo.project_members.push(ProjectMember {
            project_id: PRIMARY_PROJECT,
            user_id: ADMIN_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        // Add lookups
        repo.statuses.insert(
            1,
            crate::models::Status {
                st_id: 1,
                st_title: "Active".into(),
                st_description: "".into(),
                st_short_name: "A".into(),
            },
        );

        repo.requirement_statuses.insert(
            1,
            RequirementStatus {
                req_st_id: 1,
                req_st_title: "Draft".into(),
                req_st_description: "".into(),
                req_st_short_name: "D".into(),
            },
        );

        repo.categories.insert(
            1,
            Category {
                cat_id: 1,
                cat_title: "Systems".into(),
                cat_description: "".into(),
                cat_tag: "SYS".into(),
                project_id: PRIMARY_PROJECT,
            },
        );

        repo.verifications.insert(
            1,
            Verification {
                verification_id: 1,
                verification_name: "Analysis".into(),
                verification_description: "".into(),
                project_id: PRIMARY_PROJECT,
            },
        );

        repo.applicability.insert(
            1,
            Applicability {
                app_id: 1,
                app_title: "All".into(),
                app_description: "".into(),
                app_tag: "ALL".into(),
                project_id: PRIMARY_PROJECT,
            },
        );

        repo
    }

    fn sample_requirement(id: i32) -> Requirement {
        Requirement {
            req_id: id,
            req_title: format!("Requirement {id}"),
            req_description: "Test requirement".into(),
            req_verification: 1,
            req_current_status: 1,
            req_author: ADMIN_ID,
            req_reviewer: ADMIN_ID,
            req_link: "".into(),
            req_reference: format!("REQ-SYS-{id}"),
            req_category: 1,
            req_parent: 0,
            req_creation_date: timestamp(),
            req_update_date: timestamp(),
            req_deadline_date: timestamp(),
            req_applicability: 1,
            req_justification: Some("For testing".into()),
            project_id: PRIMARY_PROJECT,
        }
    }

    async fn test_client(repo: DieselRepoMock) -> Client {
        client_with_routes(repo, routes()).await
    }

    #[rocket::async_test]
    async fn show_requirements_lists_project_items() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("REQ-SYS-1"));
        assert!(body.contains("Requirement 1"));
    }

    #[rocket::async_test]
    async fn show_requirements_respects_status_filter() {
        let mut repo = base_repo();
        let mut req1 = sample_requirement(1);
        req1.req_current_status = 1;
        repo.requirements.insert(1, req1);

        let mut req2 = sample_requirement(2);
        req2.req_current_status = 2;
        req2.req_reference = "REQ-SYS-2".into();
        repo.requirements.insert(2, req2);

        let client = test_client(repo).await;

        let response =
            get_with_session(&client, "/p/1/requirements?status_filter=1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("Requirement 1"));
        assert!(!body.contains("Requirement 2"));
    }

    #[rocket::async_test]
    async fn show_requirements_respects_filter_with_empty_values() {
        let mut repo = base_repo();
        let mut req1 = sample_requirement(1);
        req1.req_current_status = 1;
        repo.requirements.insert(1, req1);

        let mut req2 = sample_requirement(2);
        req2.req_current_status = 2;
        req2.req_reference = "REQ-SYS-2".into();
        repo.requirements.insert(2, req2);

        let client = test_client(repo).await;

        let response = get_with_session(
            &client,
            "/p/1/requirements?status_filter=1&verification_filter=&category_filter=",
            ADMIN_ID,
        )
        .await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("Requirement 1"));
        assert!(!body.contains("Requirement 2"));
    }

    #[rocket::async_test]
    async fn show_requirements_ignores_search_query_when_filtering() {
        let mut repo = base_repo();
        let mut req1 = sample_requirement(1);
        req1.req_current_status = 1;
        repo.requirements.insert(1, req1);

        let mut req2 = sample_requirement(2);
        req2.req_current_status = 2;
        req2.req_reference = "REQ-SYS-2".into();
        repo.requirements.insert(2, req2);

        let client = test_client(repo).await;

        let response = get_with_session(
            &client,
            "/p/1/requirements?status_filter=1&verification_filter=&category_filter=&search=",
            ADMIN_ID,
        )
        .await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("Requirement 1"));
        assert!(!body.contains("Requirement 2"));
    }

    #[rocket::async_test]
    async fn show_requirements_uses_route_project_for_selected_id() {
        let mut repo = base_repo();

        // Add a second project so that the cookie can point to a different project than the route.
        repo.projects.insert(
            2,
            Project {
                project_id: 2,
                project_name: "Other Project".into(),
                project_description: Some("Alt".into()),
                project_creation_date: Some(timestamp()),
                project_update_date: Some(timestamp()),
                project_status: Some("Active".into()),
                project_owner_id: Some(ADMIN_ID),
            },
        );

        repo.project_members.push(ProjectMember {
            project_id: 2,
            user_id: ADMIN_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        let client = test_client(repo).await;

        let response = client
            .get("/p/2/requirements")
            .cookie(Cookie::new("selected_project_id", "1"))
            .private_cookie(session_cookie(ADMIN_ID))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(
            body.contains("action=\"/p/2/requirements\""),
            "filter form must target the route project"
        );
        assert!(
            !body.contains("action=\"/p/1/requirements\""),
            "filter form must not target cookie project"
        );
        assert!(
            body.contains("/p/2/requirements/new"),
            "primary action must use the route project"
        );
        assert!(
            !body.contains("/p/1/requirements/new"),
            "primary action must not use cookie project"
        );
    }

    #[rocket::async_test]
    async fn show_requirement_by_id_displays_details() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/show/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("REQ-SYS-1"));
        assert!(body.contains("For testing"));
    }

    #[rocket::async_test]
    async fn show_requirement_by_id_redirects_on_project_mismatch() {
        let mut repo = base_repo();
        let mut req = sample_requirement(1);
        req.project_id = 2;
        repo.requirements.insert(1, req);
        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/show/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/2/requirements")
        );
    }

    #[rocket::async_test]
    async fn new_requirement_form_renders() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/requirements/new", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("New Requirement"));
        assert!(body.contains("Save &amp; Add Another"));
    }

    #[rocket::async_test]
    async fn post_requirement_creates_new_entry() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/requirements/new",
            "req_title=Test&req_description=Description&req_verification=1&\
             req_current_status=1&req_author=1&req_reviewer=1&req_link=&\
             req_category=1&req_parent=0&req_applicability=1&req_reference=&\
             req_justification=Testing&project_id=1",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        let state = client.rocket().state::<TestAppState>().expect("state");
        let reqs = state
            .repo_read()
            .get_requirements_by_project(PRIMARY_PROJECT)
            .unwrap();
        assert_eq!(reqs.len(), 1);
        assert!(reqs[0].req_reference.starts_with("REQ-SYS-"));
    }

    #[rocket::async_test]
    async fn post_requirement_add_another_redirects_to_form() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/requirements/new",
            "req_title=Next+Requirement&req_description=Body&req_verification=1&\
             req_current_status=1&req_author=1&req_reviewer=1&req_link=&\
             req_category=1&req_parent=0&req_applicability=1&req_reference=&\
             req_justification=&project_id=1&intent=add_another",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/requirements/new?created=1")
        );
    }

    #[rocket::async_test]
    async fn inline_category_creation_returns_json() {
        let client = test_client(base_repo()).await;
        let response = client
            .post("/p/1/requirements/inline/category")
            .header(ContentType::JSON)
            .private_cookie(session_cookie(ADMIN_ID))
            .body(r#"{"title":"Telemetry","description":"Data channel","tag":"TEL"}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("inline response");
        let value: JsonValue = serde_json::from_str(&body).expect("json");
        assert_eq!(value["label"], "Telemetry");
        assert!(value["id"].as_i64().is_some());
    }

    #[rocket::async_test]
    async fn inline_applicability_creation_returns_json() {
        let client = test_client(base_repo()).await;
        let response = client
            .post("/p/1/requirements/inline/applicability")
            .header(ContentType::JSON)
            .private_cookie(session_cookie(ADMIN_ID))
            .body(r#"{"title":"Mission","description":"Applies to mission","tag":"MIS"}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("inline response");
        let value: JsonValue = serde_json::from_str(&body).expect("json");
        assert_eq!(value["label"], "Mission");
    }

    #[rocket::async_test]
    async fn inline_verification_creation_returns_json() {
        let client = test_client(base_repo()).await;
        let response = client
            .post("/p/1/requirements/inline/verification")
            .header(ContentType::JSON)
            .private_cookie(session_cookie(ADMIN_ID))
            .body(r#"{"name":"Inspection","description":"Visual inspection"}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("inline response");
        let value: JsonValue = serde_json::from_str(&body).expect("json");
        assert_eq!(value["label"], "Inspection");
    }

    #[rocket::async_test]
    async fn edit_requirement_form_shows_existing_data() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/edit/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("Edit Requirement"));
        assert!(body.contains("REQ-SYS-1"));
    }

    #[rocket::async_test]
    async fn post_edit_requirement_updates_existing() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = post_form_with_session(
            &client,
            "/p/1/requirements/edit/1",
            "req_id=1&req_title=Updated&req_description=New+desc&req_verification=1&\
             req_current_status=1&req_author=1&req_reviewer=1&req_link=&\
             req_category=1&req_parent=0&req_applicability=1&\
             req_justification=Changed&project_id=1&req_reference=REQ-SYS-1",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        let state = client.rocket().state::<TestAppState>().expect("state");
        let req = state.repo_read().get_requirement_by_id(1).unwrap();
        assert_eq!(req.req_title, "Updated");
        assert_eq!(req.req_description, "New desc");
    }

    #[rocket::async_test]
    async fn delete_requirement_removes_draft() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = delete_with_session(&client, "/p/1/requirements/delete/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::SeeOther);

        let state = client.rocket().state::<TestAppState>().expect("state");
        let reqs = state
            .repo_read()
            .get_requirements_by_project(PRIMARY_PROJECT)
            .unwrap();
        assert!(reqs.is_empty());
    }

    #[rocket::async_test]
    async fn delete_requirement_forbids_non_draft() {
        let mut repo = base_repo();
        let mut req = sample_requirement(1);
        req.req_current_status = 3; // Released
        repo.requirements.insert(1, req);

        // Use non-admin user
        let mut non_admin = DieselRepoMock::make_user(2, "user", "");
        non_admin.is_admin = false;
        repo.users.insert(2, non_admin);

        let client = test_client(repo).await;

        // Use non-admin cookie
        let response = client
            .delete("/p/1/requirements/delete/1")
            .private_cookie(session_cookie(2))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn show_requirements_tree_displays_hierarchy() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let mut child = sample_requirement(2);
        child.req_parent = 1;
        repo.requirements.insert(2, child);
        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/tree", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("REQ-SYS-1"));
        assert!(body.contains("REQ-SYS-2"));
    }
}
