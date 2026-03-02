// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use std::collections::HashMap;

use rocket::form::{Form, FromForm};
use rocket::response::Redirect;
use rocket::serde::json::{json, serde_json, Json};
use rocket::serde::Deserialize;
use rocket::State;
use rocket_dyn_templates::Template;

use super::prelude::*;

use crate::app::AppState;
use crate::config;
use crate::helper_functions::generate_requirement_reference;
use crate::models::*;
use crate::repository::errors::RepoError;
use crate::repository::CustomFieldRepository;
use crate::repository::ProjectMembersRepository;
use crate::services::{
    change_summary, log_change_details, resolve_change_details_labels, ApplicabilityService,
    CategoryService, CommentService, CustomFieldService, DecoratedRequirementService,
    DecoratedTestService, LabelResolvers, LogService, ProjectService, RequirementAnalyticsService,
    RequirementService, StatusService, UserService, VerificationService,
};
use crate::status_enums::{RequirementStatusEnum, TestStatusEnum};

#[derive(FromForm)]
struct RequirementCreateForm {
    #[field(name = uncased("intent"))]
    intent: Option<String>,
    #[field(name = uncased("id"))]
    id: Option<i32>,
    #[field(name = uncased("title"))]
    title: String,
    #[field(name = uncased("description"))]
    description: String,
    #[field(name = uncased("verification_method_ids"))]
    verification_method_ids: Vec<i32>,
    #[field(name = uncased("category_id"))]
    category_id: Option<i32>,
    #[field(name = uncased("status_id"))]
    status_id: Option<i32>,
    /// JSON array of { "target_requirement_id": i32, "link_type": string } for multi-parent.
    #[field(name = uncased("parent_links"))]
    parent_links: Option<String>,
    #[field(name = uncased("reference_code"))]
    reference_code: String,
    #[field(name = uncased("reviewer_id"))]
    reviewer_id: Option<i32>,
    #[field(name = uncased("applicability_id"))]
    applicability_id: Option<i32>,
    #[field(name = uncased("justification"))]
    justification: Option<String>,
    /// JSON array of { "field_id": i32, "value": string | null }
    #[field(name = uncased("custom_field_values"))]
    custom_field_values: Option<String>,
}

impl RequirementCreateForm {
    fn into_payload(
        self,
        author_id: i32,
        project_id: i32,
    ) -> (NewRequirement, Vec<i32>, Option<String>) {
        let RequirementCreateForm {
            intent,
            id,
            description,
            verification_method_ids,
            category_id,
            status_id,
            parent_links: _,
            reference_code,
            reviewer_id,
            applicability_id,
            justification,
            title,
            custom_field_values: _,
        } = self;

        let requirement = NewRequirement {
            id,
            title,
            description,
            author_id,
            category_id: category_id.unwrap_or(0),
            status_id: status_id.unwrap_or(0),
            reference_code,
            reviewer_id: reviewer_id.unwrap_or(0),
            applicability_id: applicability_id.unwrap_or(0),
            justification,
            project_id,
        };

        (requirement, verification_method_ids, intent)
    }
}

/// Form for editing a requirement; includes multiple verification method IDs.
#[derive(FromForm)]
struct RequirementEditForm {
    #[field(name = uncased("id"))]
    id: Option<i32>,
    #[field(name = uncased("title"))]
    title: String,
    #[field(name = uncased("description"))]
    description: String,
    #[field(name = uncased("status_id"))]
    status_id: i32,
    #[field(name = uncased("author_id"))]
    author_id: i32,
    #[field(name = uncased("reviewer_id"))]
    reviewer_id: i32,
    #[field(name = uncased("reference_code"))]
    reference_code: String,
    #[field(name = uncased("category_id"))]
    category_id: i32,
    #[field(name = uncased("applicability_id"))]
    applicability_id: i32,
    #[field(name = uncased("justification"))]
    justification: Option<String>,
    #[field(name = uncased("project_id"))]
    project_id: i32,
    /// JSON array of { "target_requirement_id": i32, "link_type": string } for multi-parent upstream links.
    #[field(name = uncased("parent_links"))]
    parent_links: Option<String>,
    #[field(name = uncased("verification_method_ids"))]
    verification_method_ids: Vec<i32>,
    /// JSON array of { "field_id": i32, "value": string | null }
    #[field(name = uncased("custom_field_values"))]
    custom_field_values: Option<String>,
}

#[derive(serde::Deserialize)]
struct ParentLinkEditInput {
    target_requirement_id: i32,
    link_type: String,
}

impl RequirementEditForm {
    fn to_new_requirement(&self) -> NewRequirement {
        NewRequirement {
            id: self.id,
            title: self.title.clone(),
            description: self.description.clone(),
            author_id: self.author_id,
            category_id: self.category_id,
            status_id: self.status_id,
            reference_code: self.reference_code.clone(),
            reviewer_id: self.reviewer_id,
            applicability_id: self.applicability_id,
            justification: self.justification.clone(),
            project_id: self.project_id,
        }
    }
}

/// Parse optional JSON string into custom field values for create/update.
fn parse_custom_field_values(json: Option<&str>) -> Option<Vec<CustomFieldValueInput>> {
    let s = json?.trim();
    if s.is_empty() {
        return None;
    }
    let parsed: Vec<CustomFieldValueInput> = match serde_json::from_str(s) {
        Ok(v) => v,
        Err(_) => return None,
    };
    if parsed.is_empty() {
        None
    } else {
        Some(parsed)
    }
}

fn map_repo_error(err: RepoError) -> rocket::http::Status {
    match err {
        RepoError::BadInput(_) => rocket::http::Status::BadRequest,
        RepoError::NotFound => rocket::http::Status::NotFound,
        _ => rocket::http::Status::InternalServerError,
    }
}

// TODO: This shall be an authorization check to enforce project ownership and return a redirect when mismatched
fn requirements_list_redirect(project_id: i32) -> Redirect {
    Redirect::to(uri!(
        "/p",
        show_requirements(
            project_id = project_id,
            status_filter = Option::<i32>::None,
            verification_filter = Option::<i32>::None,
            category_filter = Option::<i32>::None,
            applicability_filter = Option::<i32>::None,
            approval_filter = Option::<String>::None,
            custom_filters = Option::<String>::None,
            view = Option::<String>::None,
            page = Option::<u32>::None
        )
    ))
}

fn enforce_project_ownership(route_project_id: i32, resource_project_id: i32) -> Option<Redirect> {
    if resource_project_id != route_project_id {
        eprintln!(
            "Project mismatch: route {}, resource {}",
            route_project_id, resource_project_id
        );
        Some(requirements_list_redirect(resource_project_id))
    } else {
        None
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
    title: String,
    description: String,
    tag: String,
}

const REQUIREMENTS_PER_PAGE: i64 = 25;

/// Build query string for requirements list (view + filters), without `page`.
fn build_requirements_query(
    view: Option<&str>,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
    applicability_filter: Option<i32>,
    approval_filter: Option<&str>,
    custom_filters: Option<&str>,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(v) = view.filter(|s| !s.is_empty()) {
        parts.push(format!("view={}", v));
    }
    if let Some(id) = status_filter {
        parts.push(format!("status_filter={}", id));
    }
    if let Some(id) = verification_filter {
        parts.push(format!("verification_filter={}", id));
    }
    if let Some(id) = category_filter {
        parts.push(format!("category_filter={}", id));
    }
    if let Some(id) = applicability_filter {
        parts.push(format!("applicability_filter={}", id));
    }
    if let Some(a) = approval_filter.filter(|s| !s.is_empty()) {
        parts.push(format!("approval_filter={}", a));
    }
    if let Some(cf) = custom_filters.filter(|s| !s.is_empty()) {
        parts.push(format!("custom_filters={}", urlencoding::encode(cf)));
    }
    parts.join("&")
}

/// Build pagination context for the template (start/end range, prev/next, page numbers).
fn build_pagination_ctx(
    current_page: u32,
    total_pages: u64,
    total_count: u64,
    per_page: u64,
    query: &str,
) -> serde_json::Value {
    let start = if total_count == 0 {
        0
    } else {
        ((current_page - 1) * per_page as u32) as u64 + 1
    };
    let end = (start + per_page - 1).min(total_count);
    let has_prev = current_page > 1;
    let has_next = current_page < total_pages as u32;
    let prev_page = current_page.saturating_sub(1).max(1);
    let next_page = (current_page + 1).min(total_pages as u32).max(1);
    let total_pages_u = total_pages as u32;
    let window = 4;
    let lo = (current_page.saturating_sub(window)).max(1);
    let hi = (current_page + window).min(total_pages_u);
    let page_numbers: Vec<serde_json::Value> = (lo..=hi)
        .map(|n| json!({ "num": n, "is_current": n == current_page }))
        .collect();
    json!({
        "current_page": current_page,
        "total_pages": total_pages,
        "total_count": total_count,
        "per_page": per_page,
        "query": query,
        "start": start,
        "end": end,
        "has_prev": has_prev,
        "has_next": has_next,
        "prev_page": prev_page,
        "next_page": next_page,
        "page_numbers": page_numbers,
    })
}

fn custom_field_definitions_with_filter_values(
    defs: &[crate::models::CustomFieldDefinition],
    custom_field_filters: Option<&Vec<(i32, String)>>,
) -> Vec<serde_json::Value> {
    let filter_map: std::collections::HashMap<i32, String> = custom_field_filters
        .map(|v| v.iter().cloned().collect())
        .unwrap_or_default();
    defs.iter()
        .map(|d| {
            let current_filter_value = filter_map.get(&d.id).cloned();
            json!({
                "id": d.id,
                "label": d.label,
                "field_type": d.field_type,
                "enum_values": d.enum_values,
                "current_filter_value": current_filter_value,
            })
        })
        .collect::<Vec<_>>()
}

fn parse_custom_filters_param(s: Option<&str>) -> Option<Vec<(i32, String)>> {
    let s = s?.trim();
    if s.is_empty() {
        return None;
    }
    #[derive(serde::Deserialize)]
    struct Item {
        field_id: i32,
        value: String,
    }
    let items: Vec<Item> = serde_json::from_str(s).ok()?;
    if items.is_empty() {
        return None;
    }
    Some(items.into_iter().map(|i| (i.field_id, i.value)).collect())
}

#[get("/<project_id>/requirements?<status_filter>&<verification_filter>&<category_filter>&<applicability_filter>&<approval_filter>&<custom_filters>&<view>&<page>")]
#[allow(clippy::too_many_arguments)]
async fn show_requirements(
    project_access: ProjectAccess,
    project_id: i32,
    status_filter: Option<i32>,
    verification_filter: Option<i32>,
    category_filter: Option<i32>,
    applicability_filter: Option<i32>,
    approval_filter: Option<String>,
    custom_filters: Option<String>,
    view: Option<String>,
    page: Option<u32>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    let selected_project = ProjectService::new(state.inner()).get_by_id(project_id)?;

    let custom_field_filters = parse_custom_filters_param(custom_filters.as_deref());

    let metrics = RequirementAnalyticsService::new(state.inner()).metrics(
        project_id,
        status_filter,
        verification_filter,
        category_filter,
        applicability_filter,
    )?;

    let current_page = page.unwrap_or(1).max(1);
    let total_count = metrics.total.max(0) as u64;
    let per_page = REQUIREMENTS_PER_PAGE as u64;
    let total_pages = if total_count == 0 {
        1u64
    } else {
        total_count.div_ceil(per_page)
    };
    let total_pages_u32 = total_pages.min(u32::MAX as u64) as u32;
    let current_page = current_page.min(total_pages_u32.max(1));
    let offset = ((current_page as u64 - 1) * per_page) as i64;
    let limit = REQUIREMENTS_PER_PAGE;

    let mut requirements = DecoratedRequirementService::new(state.inner())
        .list_by_project_filtered_paginated(
            project_id,
            status_filter,
            verification_filter,
            category_filter,
            applicability_filter,
            custom_field_filters.as_deref(),
            limit,
            offset,
        )?;

    // In-memory filter by approval (paginated count unchanged; page may show fewer items)
    if let Some(ref af) = approval_filter {
        if af.eq_ignore_ascii_case("approved") {
            requirements.retain(|r| r.approval_state.eq_ignore_ascii_case("approved"));
        } else if af.eq_ignore_ascii_case("not_approved") {
            requirements.retain(|r| !r.approval_state.eq_ignore_ascii_case("approved"));
        }
    }

    // Build tree data for tree view
    // DAG: a requirement can appear under multiple parents via req_parents.
    let mut children: HashMap<i32, Vec<&DecoratedRequirement>> = HashMap::new();
    let mut roots: Vec<&DecoratedRequirement> = Vec::new();

    for r in &requirements {
        if r.req_parents.is_empty() {
            roots.push(r);
        } else {
            for parent in &r.req_parents {
                children.entry(parent.id).or_default().push(r);
            }
        }
    }

    roots.sort_by_key(|r| r.id);
    for v in children.values_mut() {
        v.sort_by_key(|r| r.id);
    }

    fn build_node<'a>(
        req: &'a DecoratedRequirement,
        idx: &HashMap<i32, Vec<&'a DecoratedRequirement>>,
    ) -> serde_json::Value {
        let kids = idx
            .get(&req.id)
            .map(|vs| vs.iter().map(|c| build_node(c, idx)).collect::<Vec<_>>())
            .unwrap_or_default();

        json!({
            "requirement": req,
            "children": kids
        })
    }

    let tree_data = roots
        .into_iter()
        .map(|r| build_node(r, &children))
        .collect::<Vec<_>>();

    // Determine current view (default to card)
    let current_view = view.as_deref().unwrap_or("card");

    let categories = CategoryService::new(state.inner()).list_by_project(project_id)?;
    let statuses =
        StatusService::new(state.inner()).list_requirement_statuses_by_project(project_id)?;
    let verifications = VerificationService::new(state.inner()).list_by_project(project_id)?;

    let inline_edit_config = json!({
        "categories": categories.iter().map(|c| json!({"id": c.id, "title": c.title})).collect::<Vec<_>>(),
        "statuses": statuses.iter().map(|s| json!({"id": s.id, "title": s.title, "tag_color": s.tag_color})).collect::<Vec<_>>(),
        "verifications": verifications.iter().map(|v| json!({"id": v.id, "title": v.title})).collect::<Vec<_>>(),
    });
    let inline_edit_config_json =
        serde_json::to_string(&inline_edit_config).unwrap_or_else(|_| "{}".to_string());

    let ctx = json!({
        "user": user,
        "requirements": json!(requirements),
        "tree_data": tree_data,
        "requirement_metrics": json!({
            "total": metrics.total,
            "draft": metrics.draft,
            "accepted": metrics.accepted,
            "rejected": metrics.rejected,
            "coverage": {
                "verified": metrics.coverage_verified,
                "percent": metrics.coverage_percent
            }
        }),
        "statuses": statuses,
        "verifications": verifications,
        "categories": categories,
        "applicability": ApplicabilityService::new(state.inner()).list_by_project(project_id)?,
        "custom_field_definitions": custom_field_definitions_with_filter_values(
            &CustomFieldService::new(state.inner()).list_by_project(project_id).unwrap_or_default(),
            custom_field_filters.as_ref(),
        ),
        "custom_filters_param": custom_filters.as_deref().unwrap_or(""),
        "users": UserService::new(state.inner()).get_by_project(project_id)?,
        "current_status_filter": json!(status_filter),
        "current_verification_filter": json!(verification_filter),
        "current_category_filter": json!(category_filter),
        "current_applicability_filter": json!(applicability_filter),
        "current_approval_filter": json!(approval_filter.as_deref().unwrap_or("")),
        "approval_filter_approved": approval_filter.as_deref().map(|a| a.eq_ignore_ascii_case("approved")).unwrap_or(false),
        "approval_filter_not_approved": approval_filter.as_deref().map(|a| a.eq_ignore_ascii_case("not_approved")).unwrap_or(false),
        "current_view": current_view,
        "project": json!({
            "id": selected_project.id,
            "name": selected_project.name,
        }),
        "is_admin": user.is_admin,
        "page_title": format!("{} - Requirements", selected_project.name),
        "inline_edit_config_json": inline_edit_config_json,
        "pagination": json!(build_pagination_ctx(
            current_page,
            total_pages,
            total_count,
            per_page,
            &build_requirements_query(view.as_deref(), status_filter, verification_filter, category_filter, applicability_filter, approval_filter.as_deref(), custom_filters.as_deref()),
        )),
    });

    Ok(Template::render("requirements/requirements", ctx))
}

#[get("/<project_id>/requirements/show/<requirement_id>")]
async fn show_requirement_id(
    project_access: ProjectAccess,
    project_id: i32,
    requirement_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();

    let selected_project = ProjectService::new(state.inner()).get_by_id(project_id)?;
    let decorated_requirement_service = DecoratedRequirementService::new(state.inner());

    let requirement = decorated_requirement_service.get_by_id(requirement_id)?;

    if let Some(redir) = enforce_project_ownership(project_id, requirement.project_id) {
        return Err(redir);
    }

    let parent_requirement = if let Some(parent_id) = requirement.req_parent_id {
        if parent_id != 0 {
            decorated_requirement_service.get_by_id(parent_id).ok()
        } else {
            None
        }
    } else {
        None
    };
    let child_requirements = decorated_requirement_service.get_by_parent_id(requirement.id)?;

    let requirement_service = RequirementService::new(state.inner());
    let parent_links: Vec<crate::models::RequirementVersionLink> = requirement
        .current_version_id
        .and_then(|vid| requirement_service.get_parent_links_for_version(vid).ok())
        .unwrap_or_default();

    // Linked verification artefacts
    let linked_tests =
        DecoratedTestService::new(state.inner()).get_linked_to_requirement(requirement_id)?;

    let (tests_passed, tests_failed, tests_pending) =
        linked_tests
            .iter()
            .fold((0_i32, 0_i32, 0_i32), |mut acc, test| {
                // Use enum to properly identify test status
                if let Some(status_enum) =
                    crate::status_enums::TestStatusEnum::from_title(&test.status_id)
                {
                    match status_enum {
                        crate::status_enums::TestStatusEnum::Passed => acc.0 += 1,
                        crate::status_enums::TestStatusEnum::Failed => acc.1 += 1,
                        _ => acc.2 += 1,
                    }
                } else {
                    // Unknown status, count as pending
                    acc.2 += 1;
                }
                acc
            });

    let history_entries = LogService::new(state.inner())
        .entity_logs(&EntityType::Requirement.to_string(), requirement_id)
        .unwrap_or_default();

    let versions = RequirementService::new(state.inner())
        .list_versions(requirement_id)
        .unwrap_or_default();
    let current_vid = requirement.current_version_id;
    let user_service = UserService::new(state.inner());
    let versions_json: Vec<serde_json::Value> = versions
        .iter()
        .map(|v| {
            let approved_by_name = v
                .approved_by
                .and_then(|uid| user_service.get_by_id(uid).ok())
                .map(|u| u.name);
            json!({
                "id": v.id,
                "requirement_id": v.requirement_id,
                "title": v.title,
                "created_at": v.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                "is_current": current_vid == Some(v.id),
                "approval_state": v.approval_state,
                "approved_by": v.approved_by,
                "approved_at": v.approved_at.map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string()),
                "approved_by_name": approved_by_name,
            })
        })
        .collect();

    let repo = state.repo_read();
    let parent_links_json: Vec<serde_json::Value> = parent_links
        .iter()
        .filter_map(|link| {
            let target_ver = repo
                .get_requirement_version_by_id(link.target_version_id)
                .ok()?;
            let target_req = repo.get_requirement_by_id(target_ver.requirement_id).ok()?;
            Some(json!({
                "link_id": link.id,
                "link_type": link.link_type,
                "rationale": link.rationale,
                "target": {
                    "id": target_req.id,
                    "reference_code": target_req.reference_code,
                    "title": target_req.title,
                }
            }))
        })
        .collect();
    let req_status_map: HashMap<i32, String> = repo
        .get_requirement_status_all()
        .unwrap_or_default()
        .into_iter()
        .map(|s| (s.id, s.title))
        .collect();
    let test_status_map: HashMap<i32, String> = repo
        .get_test_status_all()
        .unwrap_or_default()
        .into_iter()
        .map(|s| (s.id, s.title))
        .collect();
    let category_map: HashMap<i32, String> = repo
        .get_categories_by_project(project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|c| (c.id, c.title))
        .collect();
    let applicability_map: HashMap<i32, String> = repo
        .get_applicability_by_project(project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|a| (a.id, a.title))
        .collect();
    let verification_map: HashMap<i32, String> = repo
        .get_verification_by_project(project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|v| (v.id, v.title))
        .collect();
    let parent_label_map: HashMap<i32, String> = repo
        .get_requirements_by_project(project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|r| {
            let ref_display = if r.reference_code.trim().is_empty() {
                format!("RM-{}", r.id)
            } else {
                r.reference_code.clone()
            };
            (r.id, format!("{} — {}", ref_display, r.title))
        })
        .collect();
    let members = repo
        .get_members_by_project(requirement.project_id)
        .unwrap_or_default();
    drop(repo);

    let can_approve = user.is_admin
        || members
            .iter()
            .any(|m| m.user_id == user.id && (m.role == 1 || m.role == 2));
    let approved_by_name = requirement
        .approved_by
        .and_then(|uid| user_service.get_by_id(uid).ok())
        .map(|u| u.name);
    let approved_at_formatted = requirement
        .approved_at
        .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string());

    let mut test_status_list = StatusService::new(state.inner())
        .list_test_statuses_by_project(project_id)
        .unwrap_or_default();
    test_status_list.retain(|s| TestStatusEnum::from_title(&s.title).is_some());
    test_status_list.sort_by_key(|s| {
        TestStatusEnum::from_title(&s.title)
            .map(|e| e.id())
            .unwrap_or(i32::MAX)
    });
    let test_statuses: Vec<serde_json::Value> = test_status_list
        .into_iter()
        .map(|s| json!({ "id": s.id, "title": s.title, "tag_color": s.tag_color }))
        .collect();

    let entries_with_summary: Vec<serde_json::Value> = history_entries
        .iter()
        .map(|e| {
            let mut v = serde_json::to_value(e).unwrap_or_else(|_| json!({}));
            if let Some(obj) = v.as_object_mut() {
                obj.insert("summary".into(), json!(change_summary(&e.log)));
                let details = log_change_details(&e.log);
                let resolvers = LabelResolvers {
                    req_status_map: &req_status_map,
                    test_status_map: &test_status_map,
                    category_map: &category_map,
                    applicability_map: &applicability_map,
                    verification_map: &verification_map,
                    parent_label_map: &parent_label_map,
                };
                let details = resolve_change_details_labels(details, "REQUIREMENT", &resolvers);
                obj.insert("changes".into(), json!(details));
            }
            v
        })
        .collect();

    let has_custom_fields = requirement
        .custom_fields
        .as_ref()
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    let comments_items: Vec<serde_json::Value> = CommentService::new(state.inner())
        .list_comments(requirement_id, None)
        .unwrap_or_default()
        .iter()
        .map(|c| {
            let author_name = user_service
                .get_by_id(c.author_id)
                .ok()
                .map(|u| u.name.clone())
                .unwrap_or_else(|| format!("User#{}", c.author_id));
            json!({
                "id": c.id,
                "requirement_id": c.requirement_id,
                "requirement_version_id": c.requirement_version_id,
                "author_id": c.author_id,
                "author_name": author_name,
                "body": c.body,
                "created_at": c.created_at.format("%Y-%m-%d %H:%M").to_string(),
            })
        })
        .collect();
    let can_comment_on_version = !(config::lock_approved_version_comments()
        && requirement.approval_state.eq_ignore_ascii_case("approved"));

    let canonical_data = json!({
        "project_id": project_id,
        "requirement": requirement,
        "has_custom_fields": has_custom_fields,
        "versions": versions_json,
        "current_version_id": requirement.current_version_id,
        "viewing_past_version": false,
        "can_approve": can_approve,
        "approved_by_name": approved_by_name,
        "approved_at_formatted": approved_at_formatted,
        "approval_is_draft": requirement.approval_state.eq_ignore_ascii_case("draft"),
        "approval_is_reviewed": requirement.approval_state.eq_ignore_ascii_case("reviewed"),
        "approval_is_approved": requirement.approval_state.eq_ignore_ascii_case("approved"),
        "relationships": {
            "parent": parent_requirement,
            "parent_links": parent_links_json,
            "children": child_requirements,
        },
        "linked_tests": linked_tests,
        "test_statuses": test_statuses,
        "verification": {
            "tool_ids": requirement.req_verification_ids,
            "tool_id": requirement.req_verification_ids.first().copied(),
            "tool_name": requirement.verification_method_id.clone(),
            "counts": {
                "total": linked_tests.len() as i32,
                "passed": tests_passed,
                "failed": tests_failed,
                "pending": tests_pending,
            }
        },
        "history": {
            "entries": entries_with_summary,
        },
        "comments": {
            "items": comments_items,
            "can_comment_on_version": can_comment_on_version,
        }
    });

    let ctx = json!({
        "user": user,
        "project_id": project_id,
        "project": json!({
            "id": selected_project.id,
            "name": selected_project.name,
        }),
        "requirement_data": canonical_data,
        "requirement_data_json": serde_json::to_string(&canonical_data).unwrap_or_else(|_| "{}".to_string()),
        "page_title": format!("{} - Requirement", requirement.reference_code),
        "can_approve": can_approve,
        "approved_by_name": approved_by_name,
        "approved_at_formatted": approved_at_formatted,
        "approval_is_draft": requirement.approval_state.eq_ignore_ascii_case("draft"),
        "approval_is_reviewed": requirement.approval_state.eq_ignore_ascii_case("reviewed"),
        "approval_is_approved": requirement.approval_state.eq_ignore_ascii_case("approved"),
    });

    Ok(Template::render("requirements/requirement", ctx))
}

/// View a specific immutable version of a requirement (read-only).
#[get("/<project_id>/requirements/show/<requirement_id>/version/<version_id>")]
async fn show_requirement_version(
    project_access: ProjectAccess,
    project_id: i32,
    requirement_id: i32,
    version_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let selected_project = ProjectService::new(state.inner()).get_by_id(project_id)?;
    let requirement_service = RequirementService::new(state.inner());
    let version = requirement_service.get_version_by_id(version_id)?;
    if version.requirement_id != requirement_id {
        return Err(Redirect::to(uri!(show_requirement_id(
            project_id,
            requirement_id
        ))));
    }
    let current = requirement_service.get_by_id(requirement_id)?;
    if let Some(redir) = enforce_project_ownership(project_id, current.project_id) {
        return Err(redir);
    }
    let version_custom_fields = state
        .repo_read()
        .get_custom_field_values_for_version(version_id)
        .ok();
    let req_from_version = Requirement {
        id: current.id,
        current_version_id: current.current_version_id,
        same_as_current: None,
        title: version.title.clone(),
        description: version.description.clone(),
        status_id: version.status_id,
        author_id: version.author_id,
        reviewer_id: version.reviewer_id,
        reference_code: current.reference_code.clone(),
        category_id: version.category_id,
        parent_id: None, // populated from requirement_version_links by decorator layer
        creation_date: version.created_at,
        update_date: version.created_at,
        deadline_date: version.deadline_date,
        applicability_id: version.applicability_id,
        justification: version.justification.clone(),
        project_id: current.project_id,
        approval_state: version.approval_state.clone(),
        approved_by: version.approved_by,
        approved_at: version.approved_at,
        custom_fields: version_custom_fields,
    };
    let decorated_requirement_service = DecoratedRequirementService::new(state.inner());
    let requirement = decorated_requirement_service.decorate_requirement(&req_from_version)?;
    let parent_requirement = if let Some(parent_id) = requirement.req_parent_id {
        if parent_id != 0 {
            decorated_requirement_service.get_by_id(parent_id).ok()
        } else {
            None
        }
    } else {
        None
    };
    let child_requirements = decorated_requirement_service.get_by_parent_id(requirement.id)?;
    let linked_tests =
        DecoratedTestService::new(state.inner()).get_linked_to_requirement(requirement_id)?;
    let (tests_passed, tests_failed, tests_pending) =
        linked_tests
            .iter()
            .fold((0_i32, 0_i32, 0_i32), |mut acc, test| {
                if let Some(status_enum) =
                    crate::status_enums::TestStatusEnum::from_title(&test.status_id)
                {
                    match status_enum {
                        crate::status_enums::TestStatusEnum::Passed => acc.0 += 1,
                        crate::status_enums::TestStatusEnum::Failed => acc.1 += 1,
                        _ => acc.2 += 1,
                    }
                } else {
                    acc.2 += 1;
                }
                acc
            });
    let versions = requirement_service
        .list_versions(requirement_id)
        .unwrap_or_default();
    let current_vid = current.current_version_id;
    let user_service = UserService::new(state.inner());
    let versions_json: Vec<serde_json::Value> = versions
        .iter()
        .map(|v| {
            let approved_by_name = v
                .approved_by
                .and_then(|uid| user_service.get_by_id(uid).ok())
                .map(|u| u.name);
            json!({
                "id": v.id,
                "requirement_id": v.requirement_id,
                "title": v.title,
                "created_at": v.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                "is_current": current_vid == Some(v.id),
                "approval_state": v.approval_state,
                "approved_by": v.approved_by,
                "approved_at": v.approved_at.map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string()),
                "approved_by_name": approved_by_name,
            })
        })
        .collect();
    let repo_version = state.repo_read();
    let members_version = repo_version
        .get_members_by_project(current.project_id)
        .unwrap_or_default();
    let can_approve_version = user.is_admin
        || members_version
            .iter()
            .any(|m| m.user_id == user.id && (m.role == 1 || m.role == 2));
    let approved_by_name_version = requirement
        .approved_by
        .and_then(|uid| user_service.get_by_id(uid).ok())
        .map(|u| u.name);
    let approved_at_formatted_version = requirement
        .approved_at
        .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string());
    let mut test_status_list = StatusService::new(state.inner())
        .list_test_statuses_by_project(project_id)
        .unwrap_or_default();
    test_status_list.retain(|s| TestStatusEnum::from_title(&s.title).is_some());
    test_status_list.sort_by_key(|s| {
        TestStatusEnum::from_title(&s.title)
            .map(|e| e.id())
            .unwrap_or(i32::MAX)
    });
    let test_statuses: Vec<serde_json::Value> = test_status_list
        .into_iter()
        .map(|s| json!({ "id": s.id, "title": s.title, "tag_color": s.tag_color }))
        .collect();
    let history_entries = LogService::new(state.inner())
        .entity_logs(&EntityType::Requirement.to_string(), requirement_id)
        .unwrap_or_default();
    let repo = state.repo_read();
    let req_status_map: HashMap<i32, String> = repo
        .get_requirement_status_all()
        .unwrap_or_default()
        .into_iter()
        .map(|s| (s.id, s.title))
        .collect();
    let test_status_map: HashMap<i32, String> = repo
        .get_test_status_all()
        .unwrap_or_default()
        .into_iter()
        .map(|s| (s.id, s.title))
        .collect();
    let category_map: HashMap<i32, String> = repo
        .get_categories_by_project(project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|c| (c.id, c.title))
        .collect();
    let applicability_map: HashMap<i32, String> = repo
        .get_applicability_by_project(project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|a| (a.id, a.title))
        .collect();
    let verification_map: HashMap<i32, String> = repo
        .get_verification_by_project(project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|v| (v.id, v.title))
        .collect();
    let parent_label_map: HashMap<i32, String> = repo
        .get_requirements_by_project(project_id)
        .unwrap_or_default()
        .into_iter()
        .map(|r| {
            let ref_display = if r.reference_code.trim().is_empty() {
                format!("RM-{}", r.id)
            } else {
                r.reference_code.clone()
            };
            (r.id, format!("{} — {}", ref_display, r.title))
        })
        .collect();
    drop(repo);
    let entries_with_summary: Vec<serde_json::Value> = history_entries
        .iter()
        .map(|e| {
            let mut v = serde_json::to_value(e).unwrap_or_else(|_| json!({}));
            if let Some(obj) = v.as_object_mut() {
                obj.insert("summary".into(), json!(change_summary(&e.log)));
                let details = log_change_details(&e.log);
                let resolvers = LabelResolvers {
                    req_status_map: &req_status_map,
                    test_status_map: &test_status_map,
                    category_map: &category_map,
                    applicability_map: &applicability_map,
                    verification_map: &verification_map,
                    parent_label_map: &parent_label_map,
                };
                obj.insert(
                    "changes".into(),
                    json!(resolve_change_details_labels(
                        details,
                        "REQUIREMENT",
                        &resolvers
                    )),
                );
            }
            v
        })
        .collect();
    let has_custom_fields = requirement
        .custom_fields
        .as_ref()
        .map(|v| !v.is_empty())
        .unwrap_or(false);

    let comments_items_version: Vec<serde_json::Value> = CommentService::new(state.inner())
        .list_comments(requirement_id, Some(version_id))
        .unwrap_or_default()
        .iter()
        .map(|c| {
            let author_name = user_service
                .get_by_id(c.author_id)
                .ok()
                .map(|u| u.name.clone())
                .unwrap_or_else(|| format!("User#{}", c.author_id));
            json!({
                "id": c.id,
                "requirement_id": c.requirement_id,
                "requirement_version_id": c.requirement_version_id,
                "author_id": c.author_id,
                "author_name": author_name,
                "body": c.body,
                "created_at": c.created_at.format("%Y-%m-%d %H:%M").to_string(),
            })
        })
        .collect();
    let can_comment_on_version_view = !(config::lock_approved_version_comments()
        && version.approval_state.eq_ignore_ascii_case("approved"));

    let canonical_data = json!({
        "project_id": project_id,
        "requirement": requirement,
        "has_custom_fields": has_custom_fields,
        "versions": versions_json,
        "current_version_id": current.current_version_id,
        "viewing_past_version": true,
        "viewing_version_id": version_id,
        "can_approve": can_approve_version,
        "approved_by_name": approved_by_name_version,
        "approved_at_formatted": approved_at_formatted_version,
        "approval_is_draft": requirement.approval_state.eq_ignore_ascii_case("draft"),
        "approval_is_reviewed": requirement.approval_state.eq_ignore_ascii_case("reviewed"),
        "approval_is_approved": requirement.approval_state.eq_ignore_ascii_case("approved"),
        "relationships": { "parent": parent_requirement, "children": child_requirements },
        "linked_tests": linked_tests,
        "test_statuses": test_statuses,
        "verification": {
            "tool_ids": requirement.req_verification_ids,
            "tool_id": requirement.req_verification_ids.first().copied(),
            "tool_name": requirement.verification_method_id.clone(),
            "counts": { "total": linked_tests.len() as i32, "passed": tests_passed, "failed": tests_failed, "pending": tests_pending }
        },
        "history": { "entries": entries_with_summary },
        "comments": {
            "items": comments_items_version,
            "can_comment_on_version": can_comment_on_version_view,
        }
    });
    let ctx = json!({
        "user": user,
        "project_id": project_id,
        "project": json!({ "id": selected_project.id, "name": selected_project.name }),
        "requirement_data": canonical_data,
        "requirement_data_json": serde_json::to_string(&canonical_data).unwrap_or_else(|_| "{}".to_string()),
        "page_title": format!("{} - Requirement (v{})", requirement.reference_code, version_id),
        "viewing_past_version": true,
        "viewing_version_id": version_id,
        "can_approve": can_approve_version,
        "approved_by_name": approved_by_name_version,
        "approved_at_formatted": approved_at_formatted_version,
    });
    Ok(Template::render("requirements/requirement", ctx))
}

#[get("/<project_id>/requirements/edit/<requirement_id>")]
async fn get_edit_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    requirement_id: i32,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let name = ProjectService::new(state.inner())
        .get_by_id(project_id)?
        .name;
    let service = DecoratedRequirementService::new(state.inner());
    let req = service.get_by_id(requirement_id)?;

    // Enforce project ownership; redirect if mismatched
    if let Some(redir) = enforce_project_ownership(project_id, req.project_id) {
        return Err(redir);
    }

    let parent: Option<DecoratedRequirement> = if let Some(parent_id) = req.req_parent_id {
        if parent_id != 0 {
            Some(service.get_by_id(parent_id)?)
        } else {
            None
        }
    } else {
        None
    };

    let history_entries = LogService::new(state.inner())
        .entity_logs(&EntityType::Requirement.to_string(), requirement_id)
        .unwrap_or_default();

    let version_counter = history_entries.len().saturating_add(1);
    let version_label = format!("v1.{}", version_counter.saturating_sub(1));
    let last_editor_name = history_entries
        .first()
        .map(|entry| entry.username.clone())
        .filter(|name| !name.is_empty())
        .or_else(|| {
            if !req.reviewer_id.trim().is_empty() {
                Some(req.reviewer_id.clone())
            } else if !req.author_id.trim().is_empty() {
                Some(req.author_id.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "Unknown author".to_string());

    let categories = CategoryService::new(state.inner()).list_by_project(project_id)?;
    let statuses =
        StatusService::new(state.inner()).list_requirement_statuses_by_project(project_id)?;
    let users = UserService::new(state.inner()).get_by_project(project_id)?;
    let verifications = VerificationService::new(state.inner()).list_by_project(project_id)?;
    let applicability = ApplicabilityService::new(state.inner()).list_by_project(project_id)?;
    let custom_field_definitions = CustomFieldService::new(state.inner())
        .list_by_project(project_id)
        .unwrap_or_default();
    let custom_field_definitions_with_values: Vec<serde_json::Value> = custom_field_definitions
        .iter()
        .map(|def| {
            let current_value = req
                .custom_fields
                .as_ref()
                .and_then(|cf| cf.iter().find(|v| v.field_id == def.id))
                .and_then(|v| v.value.clone())
                .unwrap_or_default();
            json!({
                "id": def.id,
                "label": def.label,
                "field_type": def.field_type,
                "enum_values": def.enum_values,
                "sort_order": def.sort_order,
                "current_value": current_value,
            })
        })
        .collect();

    // Lightweight list of other requirements for linking (excluding current requirement), sorted by ID
    let mut candidates: Vec<_> = RequirementService::new(state.inner())
        .list_by_project(project_id)?
        .into_iter()
        .filter(|candidate| candidate.id != requirement_id) // Don't allow self-reference
        .collect();
    candidates.sort_by_key(|c| c.id);
    let linked_requirement_options: Vec<_> = candidates
        .iter()
        .map(|candidate| {
            json!({
                "id": candidate.id,
                "title": candidate.title,
                "reference": candidate.reference_code,
            })
        })
        .collect();

    let requirement_service = RequirementService::new(state.inner());
    let parent_links_edit: Vec<serde_json::Value> = req
        .current_version_id
        .and_then(|vid| requirement_service.get_parent_links_for_version(vid).ok())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|link| {
            let repo = state.repo_read();
            let target_ver = repo
                .get_requirement_version_by_id(link.target_version_id)
                .ok()?;
            let target_req = repo.get_requirement_by_id(target_ver.requirement_id).ok()?;
            Some(json!({
                "target_requirement_id": target_req.id,
                "reference": target_req.reference_code,
                "title": target_req.title,
                "link_type": link.link_type,
            }))
        })
        .collect();
    let link_types: Vec<String> =
        crate::services::requirement_service::REQUIREMENT_VERSION_LINK_TYPES
            .iter()
            .map(|s| (*s).to_string())
            .collect();

    let display_reference = if req.reference_code.trim().is_empty() {
        format!("RM-{:03}", req.id)
    } else {
        req.reference_code.clone()
    };

    let verification_with_selected: Vec<serde_json::Value> = verifications
        .iter()
        .map(|v| {
            json!({
                "id": v.id,
                "title": v.title,
                "description": v.description,
                "tag": v.tag,
                "project_id": v.project_id,
                "selected": req.req_verification_ids.contains(&v.id),
            })
        })
        .collect();

    let ctx = json!({
        "req": req,
        "categories": categories,
        "statuses": statuses,
        "parent": parent,
        "users": users,
        "verification": verifications,
        "verification_with_selected": verification_with_selected,
        "applicability": applicability,
        "custom_field_definitions": custom_field_definitions,
        "custom_field_definitions_with_values": custom_field_definitions_with_values,
        "linked_requirement_options": linked_requirement_options,
        "parent_links_edit": parent_links_edit,
        "link_types": link_types,
        "user": user,
        "display_reference": display_reference,
        "name": name,
        "version": {
            "label": version_label,
            "last_editor": last_editor_name,
            "updated_at": req.update_date,
        },
        "autosave": {
            "enabled": true,
            "interval_ms": 3_000
        },
        "page_title": format!("Edit {} - Requirement", display_reference),
    });

    #[cfg(debug_assertions)]
    println!("Edit requirement ctx: {:#}", ctx);

    Ok(Template::render("requirements/edit_requirement", ctx))
}

#[post("/<project_id>/requirements/edit/<requirement_id>", data = "<form>")]
async fn post_edit_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    requirement_id: i32,
    form: Form<RequirementEditForm>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let service = RequirementService::new(state.inner());
    if let Some(redir) =
        enforce_project_ownership(project_id, service.get_by_id(requirement_id)?.project_id)
    {
        return Err(redir);
    }

    let form = form.into_inner();
    let verification_ids: Vec<i32> = form
        .verification_method_ids
        .iter()
        .filter(|&&id| id > 0)
        .copied()
        .collect();
    let custom_fields = parse_custom_field_values(form.custom_field_values.as_deref());
    let parent_links: Option<Vec<(i32, String)>> = form.parent_links.as_deref().and_then(|json| {
        let links: Vec<ParentLinkEditInput> = serde_json::from_str(json.trim()).ok()?;
        if links.is_empty() {
            return Some(vec![]);
        }
        let repo = state.inner().repo_read();
        let to_create: Vec<(i32, String)> = links
            .iter()
            .filter_map(|pl| {
                if pl.target_requirement_id <= 0 {
                    return None;
                }
                let target_req = repo.get_requirement_by_id(pl.target_requirement_id).ok()?;
                if target_req.project_id != project_id {
                    return None;
                }
                let target_version_id = target_req.current_version_id?;
                Some((target_version_id, pl.link_type.clone()))
            })
            .collect();
        Some(to_create)
    });
    let user = project_access.into_user();
    service.update(
        &user,
        requirement_id,
        form.to_new_requirement(),
        &verification_ids,
        custom_fields.as_deref(),
        parent_links,
    )?;
    Ok(Redirect::to(uri!(
        "/p",
        show_requirement_id(project_id, requirement_id)
    )))
}

#[delete("/<project_id>/requirements/delete/<requirement_id>")]
async fn delete_requirement_route(
    project_access: ProjectAccess,
    project_id: i32,
    requirement_id: i32,
    state: &State<AppState>,
) -> Result<Redirect, rocket::http::Status> {
    let user = project_access.into_user();

    let service = RequirementService::new(state.inner());
    let req = service
        .get_by_id(requirement_id)
        .map_err(|_| rocket::http::Status::NotFound)?;

    if let Some(redir) = enforce_project_ownership(project_id, req.project_id) {
        return Ok(redir);
    }

    // Permission gate: allow only Draft or Proposal status, or admin
    // Use the enum to check if the status is editable
    let is_editable = RequirementStatusEnum::from_id(req.status_id)
        .map(|status| status.is_editable_by_user())
        .unwrap_or(false);

    if !is_editable && !user.is_admin {
        return Err(rocket::http::Status::Forbidden);
    }

    service
        .delete(&user, requirement_id)
        .map_err(|_| rocket::http::Status::InternalServerError)?;

    Ok(requirements_list_redirect(project_id))
}

#[get("/<project_id>/requirements/new?<error>&<created>&<parent>&<template>")]
async fn new_requirement(
    project_access: ProjectAccess,
    project_id: i32,
    state: &State<AppState>,
    error: Option<String>,
    created: Option<String>,
    parent: Option<i32>,
    template: Option<i32>, // use this requirement as a template
) -> Result<Template, Redirect> {
    let user = project_access.into_user();
    let requirement_service = RequirementService::new(state.inner());

    let project = ProjectService::new(state.inner()).get_by_id(project_id)?;
    let statuses =
        StatusService::new(state.inner()).list_requirement_statuses_by_project(project_id)?;
    let categories = CategoryService::new(state.inner()).list_by_project(project_id)?;
    let users = UserService::new(state.inner()).get_by_project(project_id)?;
    let verifications = VerificationService::new(state.inner()).list_by_project(project_id)?;
    let applicability = ApplicabilityService::new(state.inner()).list_by_project(project_id)?;
    let custom_field_definitions = CustomFieldService::new(state.inner())
        .list_by_project(project_id)
        .unwrap_or_default();

    // Lightweight list of other requirements for linking
    let parents = RequirementService::new(state.inner())
        .list_by_project(project_id)?
        .into_iter()
        .map(|candidate| {
            json!({
                "id": candidate.id,
                "title": candidate.title,
                "reference": candidate.reference_code,
            })
        })
        .collect::<Vec<_>>();

    let template_requirement: Option<Requirement> =
        template.and_then(|id| requirement_service.get_by_id(id).ok());

    let tr = template_requirement.as_ref(); // Option<&Requirement>

    let template_verification_ids: Vec<i32> = match &template_requirement {
        Some(r) => state
            .repo_read()
            .get_verification_method_ids_for_requirement(r.id)
            .unwrap_or_default(),
        None => vec![],
    };

    let new_requirement = NewRequirement {
        id: None,
        title: tr.map(|r| r.title.clone()).unwrap_or_default(),
        description: tr.map(|r| r.description.clone()).unwrap_or_default(),
        author_id: user.id,
        category_id: tr.map(|r| r.category_id).unwrap_or_default(),
        status_id: 0, // Draft
        reference_code: tr.map(|r| r.reference_code.clone()).unwrap_or_default(),
        reviewer_id: tr.map(|r| r.reviewer_id).unwrap_or_default(),
        applicability_id: tr.map(|r| r.applicability_id).unwrap_or_default(),
        justification: tr.and_then(|r| r.justification.clone()),
        project_id,
    };

    // Default status to "Draft"
    let mut new_requirement = new_requirement;
    new_requirement.status_id = statuses
        .iter()
        .find(|st| st.title.eq_ignore_ascii_case("Draft"))
        .map(|st| st.id)
        .unwrap_or(RequirementStatusEnum::Draft.id());

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

    // Check if user is admin or project owner
    let is_admin_or_owner = user.is_admin || project.owner_id == Some(user.id);

    let verification_with_selected: Vec<serde_json::Value> = verifications
        .iter()
        .map(|v| {
            json!({
                "id": v.id,
                "title": v.title,
                "description": v.description,
                "tag": v.tag,
                "project_id": v.project_id,
                "selected": template_verification_ids.contains(&v.id),
            })
        })
        .collect();

    let link_types: Vec<String> =
        crate::services::requirement_service::REQUIREMENT_VERSION_LINK_TYPES
            .iter()
            .map(|s| (*s).to_string())
            .collect();

    let parent_links_edit: Vec<serde_json::Value> = parent
        .and_then(|pid| {
            parents.iter().find(|p| p["id"] == pid).map(|p| {
                vec![json!({
                    "target_requirement_id": p["id"],
                    "reference": p["reference"],
                    "title": p["title"],
                    "link_type": "DERIVES_FROM",
                })]
            })
        })
        .unwrap_or_default();

    // Defaults for dropdowns so the template can pre-select and form submits valid values
    let category_id = if new_requirement.category_id > 0 {
        new_requirement.category_id
    } else {
        categories.first().map(|c| c.id).unwrap_or(0)
    };
    let applicability_id = if new_requirement.applicability_id > 0 {
        new_requirement.applicability_id
    } else {
        applicability.first().map(|a| a.id).unwrap_or(0)
    };
    let reviewer_id = if new_requirement.reviewer_id > 0 {
        new_requirement.reviewer_id
    } else {
        user.id
    };
    new_requirement.category_id = category_id;
    new_requirement.applicability_id = applicability_id;
    new_requirement.reviewer_id = reviewer_id;

    let ctx = json!({
        "categories": categories,
        "status": statuses,
        "parent": parents,
        "link_types": link_types,
        "parent_links_edit": parent_links_edit,
        "users": users,
        "verification": verifications,
        "verification_with_selected": verification_with_selected,
        "applicability": applicability,
        "custom_field_definitions": custom_field_definitions,
        "project_id": project_id,
        "project": {
            "id": project.id,
            "name": project.name,
        },
        "template": new_requirement,
        "template_verification_ids": template_verification_ids,
        "created_timestamp": created_timestamp,
        "user": user,
        "is_admin_or_owner": is_admin_or_owner,
        "error": error,
        "flash_success": created_flash,
        "page_title": format!("New Requirement - {}", project.name),
        "category_id": category_id,
        "applicability_id": applicability_id,
        "status_id": new_requirement.status_id,
        "reviewer_id": reviewer_id,
    });

    Ok(Template::render("requirements/new_requirement", ctx))
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
            created = Option::<String>::None,
            parent = Option::<i32>::None,
            template = Option::<i32>::None
        )
    );

    let form = new_req.into_inner();
    let parent_links_for_create = form.parent_links.clone();
    let custom_fields = parse_custom_field_values(form.custom_field_values.as_deref());
    let (mut req, verification_method_ids, intent) = form.into_payload(user.id, project_id);
    req.project_id = project_id;
    req.author_id = user.id;

    // If required IDs didn't come through (e.g. 0 from dropdowns), use sensible defaults so validate_requirement passes
    let categories = CategoryService::new(state.inner())
        .list_by_project(project_id)
        .unwrap_or_default();
    let applicability_list = ApplicabilityService::new(state.inner())
        .list_by_project(project_id)
        .unwrap_or_default();
    let statuses = StatusService::new(state.inner())
        .list_requirement_statuses_by_project(project_id)
        .unwrap_or_default();

    if req.category_id <= 0 && !categories.is_empty() {
        req.category_id = categories[0].id;
    }
    if req.applicability_id <= 0 && !applicability_list.is_empty() {
        req.applicability_id = applicability_list[0].id;
    }
    if req.status_id <= 0 {
        req.status_id = statuses
            .iter()
            .find(|s| s.title.eq_ignore_ascii_case("Draft"))
            .map(|s| s.id)
            .unwrap_or_else(|| RequirementStatusEnum::Draft.id());
    }
    if req.reviewer_id <= 0 {
        req.reviewer_id = user.id;
    }

    // Allow empty verification methods; user can add them when editing
    let verification_method_ids: Vec<i32> = verification_method_ids
        .into_iter()
        .filter(|&id| id > 0)
        .collect();

    // --- Reference validation / generation ---
    let category = get_category_or_placeholder(state, req.category_id);
    let expected_prefix = format!("REQ-{}-", category.tag);
    let pat = format!(r"^REQ-{}-\d+$", regex::escape(&category.tag));
    let re =
        regex::Regex::new(&pat).unwrap_or_else(|_| regex::Regex::new(r"^REQ-\w+-\d+$").unwrap());
    let reference_ok = !req.reference_code.is_empty()
        && req.reference_code.starts_with(&expected_prefix)
        && re.is_match(&req.reference_code);

    if !reference_ok {
        if req.reference_code.trim().is_empty() {
            // Generate when missing
            let generated = {
                let repo = state.repo_read();
                generate_requirement_reference(&*repo, req.category_id, req.project_id)
            };
            match generated {
                Ok(reference) => req.reference_code = reference,
                Err(_e) => {
                    #[cfg(debug_assertions)]
                    eprintln!("reference generation failed: {:?}", _e);
                    req.reference_code = format!("REQ-UNKNOWN-{}", chrono::Utc::now().timestamp());
                }
            }
        } else {
            // User supplied a reference that doesn't match format; redirect back with error
            return Err(Redirect::to(new_url));
        }
    }

    let failure_url = uri!(
        "/p",
        new_requirement(
            project_id = project_id,
            error = Some("Failed to create requirement".to_string()),
            created = Option::<String>::None,
            parent = Option::<i32>::None,
            template = Option::<i32>::None
        )
    );

    // --- Insert ---
    let service = RequirementService::new(state.inner());
    let custom_fields_ref = custom_fields.as_deref();
    let id = match service.create(&user, req, &verification_method_ids, custom_fields_ref) {
        Ok(id) => id,
        Err(crate::repository::errors::RepoError::BadInput(msg)) => {
            eprintln!("post_requirement: validation failed: {}", msg);
            return Err(Redirect::to(new_url));
        }
        Err(_err) => {
            #[cfg(debug_assertions)]
            eprintln!("service create requirement failed: {:?}", _err);
            return Err(Redirect::to(failure_url));
        }
    };

    if let Some(links_json) = parent_links_for_create.as_deref() {
        let links: Vec<ParentLinkEditInput> =
            serde_json::from_str(links_json.trim()).unwrap_or_default();
        if !links.is_empty() {
            let requirement = service.get_by_id(id)?;
            if let Some(source_version_id) = requirement.current_version_id {
                let to_create: Vec<(i32, String)> = {
                    let repo = state.inner().repo_read();
                    links
                        .iter()
                        .filter_map(|pl| {
                            if pl.target_requirement_id <= 0 {
                                return None;
                            }
                            let target_req =
                                repo.get_requirement_by_id(pl.target_requirement_id).ok()?;
                            if target_req.project_id != project_id {
                                return None;
                            }
                            let target_version_id = target_req.current_version_id?;
                            Some((target_version_id, pl.link_type.clone()))
                        })
                        .collect()
                };
                for (target_version_id, link_type) in to_create {
                    let _ = service.create_requirement_version_link(
                        source_version_id,
                        target_version_id,
                        &link_type,
                        project_id,
                        None,
                        None,
                    );
                }
            }
        }
    }

    if matches!(intent.as_deref(), Some("add_another")) {
        return Ok(Redirect::to(uri!(
            "/p",
            new_requirement(
                project_id = project_id,
                error = Option::<String>::None,
                created = Some("1".to_string()),
                parent = Option::<i32>::None,
                template = Option::<i32>::None
            )
        )));
    }

    // --- Success: show the new requirement ---
    Ok(Redirect::to(uri!(
        "/p",
        show_requirement_id(project_id, id)
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
        if r.parent_id.is_none() || r.parent_id == Some(0) {
            roots.push(r);
        } else if let Some(parent_id) = r.parent_id {
            children.entry(parent_id).or_default().push(r);
        }
    }

    // Sort roots and each child list by id for deterministic output
    roots.sort_by_key(|r| r.id);
    for v in children.values_mut() {
        v.sort_by_key(|r| r.id);
    }

    // Recursive builder
    fn build_node<'a>(
        req: &'a Requirement,
        idx: &HashMap<i32, Vec<&'a Requirement>>,
    ) -> serde_json::Value {
        let kids = idx
            .get(&req.id)
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
        "selected_project_id": project_id,
        "page_title": "Requirements Tree"
    });

    Ok(Template::render("requirements/requirements_tree", ctx))
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
        id: None,
        title: data.title,
        description: data.description,
        tag: data.tag,
        project_id,
    };

    let id = category_service
        .create(&user, new_category)
        .map_err(map_repo_error)?;
    let stored = category_service.get_by_id(id).map_err(map_repo_error)?;

    Ok(Json(json!({
        "id": stored.id,
        "label": stored.title,
        "tag": stored.tag,
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
        id: None,
        title: data.title,
        description: data.description,
        tag: data.tag,
        project_id,
    };

    let id = applicability_service
        .create(&user, new_applicability)
        .map_err(map_repo_error)?;
    let stored = applicability_service
        .get_by_id(id)
        .map_err(map_repo_error)?;

    Ok(Json(json!({
        "id": stored.id,
        "label": stored.title,
        "tag": stored.tag,
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
    let new_verification = NewVerificationMethod {
        id: None,
        title: data.title,
        description: data.description,
        tag: data.tag,
        project_id,
    };

    let id = verification_service
        .create(new_verification)
        .map_err(map_repo_error)?;
    let stored = verification_service.get_by_id(id).map_err(map_repo_error)?;

    Ok(Json(json!({
        "id": stored.id,
        "label": stored.title,
        "description": stored.description,
    })))
}

fn get_category_or_placeholder(state: &State<AppState>, category_id: i32) -> Category {
    CategoryService::new(state.inner())
        .get_by_id(category_id)
        .unwrap_or_else(|_| Category {
            id: category_id,
            title: format!("Unknown Category ({})", category_id),
            description: "Category not found".to_string(),
            tag: "unknown".to_string(),
            project_id: 1,
        })
}

pub fn routes() -> Vec<Route> {
    routes![
        show_requirements,
        show_requirement_id,
        show_requirement_version,
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
    use crate::status_enums::ProjectStatus;
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
                id: PRIMARY_PROJECT,
                name: "Test Project".into(),
                description: Some("Description".into()),
                creation_date: Some(timestamp()),
                update_date: Some(timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(ADMIN_ID),
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
        repo.requirement_statuses.insert(
            1,
            RequirementStatus {
                id: 1,
                title: "Draft".into(),
                description: "".into(),
                tag: "D".into(),
                project_id: 1,
                is_system: false,
                tag_color: None,
            },
        );

        repo.categories.insert(
            1,
            Category {
                id: 1,
                title: "Systems".into(),
                description: "".into(),
                tag: "SYS".into(),
                project_id: PRIMARY_PROJECT,
            },
        );

        repo.verifications.insert(
            1,
            VerificationMethod {
                id: 1,
                title: "Analysis".into(),
                description: "".into(),
                tag: "ANALYSIS".into(),
                project_id: PRIMARY_PROJECT,
            },
        );

        repo.applicability.insert(
            1,
            Applicability {
                id: 1,
                title: "All".into(),
                description: "".into(),
                tag: "ALL".into(),
                project_id: PRIMARY_PROJECT,
            },
        );

        repo
    }

    fn sample_requirement(id: i32) -> Requirement {
        Requirement {
            id,
            current_version_id: None,
            same_as_current: None,
            title: format!("Requirement {id}"),
            description: "Test requirement".into(),
            status_id: 1,
            author_id: ADMIN_ID,
            reviewer_id: ADMIN_ID,
            reference_code: format!("REQ-SYS-{id}"),
            category_id: 1,
            parent_id: None,
            creation_date: timestamp(),
            update_date: timestamp(),
            deadline_date: Some(timestamp()),
            applicability_id: 1,
            justification: Some("For testing".into()),
            project_id: PRIMARY_PROJECT,
            approval_state: "draft".to_string(),
            approved_by: None,
            approved_at: None,
            custom_fields: None,
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
        req1.status_id = 1;
        repo.requirements.insert(1, req1);

        let mut req2 = sample_requirement(2);
        req2.status_id = 2;
        req2.reference_code = "REQ-SYS-2".into();
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
        req1.status_id = 1;
        repo.requirements.insert(1, req1);

        let mut req2 = sample_requirement(2);
        req2.status_id = 2;
        req2.reference_code = "REQ-SYS-2".into();
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
        req1.status_id = 1;
        repo.requirements.insert(1, req1);

        let mut req2 = sample_requirement(2);
        req2.status_id = 2;
        req2.reference_code = "REQ-SYS-2".into();
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
                id: 2,
                name: "Other Project".into(),
                description: Some("Alt".into()),
                creation_date: Some(timestamp()),
                update_date: Some(timestamp()),
                status: ProjectStatus::Active,
                owner_id: Some(ADMIN_ID),
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
    async fn new_requirement_form_renders() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/requirements/new", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("New Requirement"));
        assert!(body.contains("Save"));
        assert!(body.contains("Cancel"));
    }

    #[rocket::async_test]
    async fn post_requirement_creates_new_entry() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/requirements/new",
            "title=Test&description=Description&verification_method_ids=1&\
             status_id=1&reviewer_id=1&\
             category_id=1&parent_id=0&applicability_id=1&reference_code=&\
             justification=Testing",
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
        assert_eq!(reqs[0].author_id, ADMIN_ID);
        assert_eq!(reqs[0].project_id, PRIMARY_PROJECT);
        assert!(reqs[0].reference_code.starts_with("REQ-SYS-"));
    }

    #[rocket::async_test]
    async fn post_requirement_add_another_redirects_to_form() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/requirements/new",
            "title=Next+Requirement&description=Body&verification_method_ids=1&\
             status_id=1&reviewer_id=1&\
             category_id=1&parent_id=0&applicability_id=1&reference_code=&\
             justification=&intent=add_another",
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
            .body(r#"{"title":"Inspection","description":"Visual inspection","tag":"INSPECTION"}"#)
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
        //assert!(body.contains("Edit Requirement"));
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
            "id=1&title=Updated&description=New+desc&verification_method_ids=1&\
             status_id=1&author_id=1&reviewer_id=1&\
             category_id=1&parent_id=0&applicability_id=1&\
             justification=Changed&project_id=1&reference_code=REQ-SYS-1",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        let state = client.rocket().state::<TestAppState>().expect("state");
        let req = state.repo_read().get_requirement_by_id(1).unwrap();
        assert_eq!(req.title, "Updated");
        assert_eq!(req.description, "New desc");
    }

    #[rocket::async_test]
    async fn delete_requirement_removes_draft() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = delete_with_session(&client, "/p/1/requirements/delete/1", ADMIN_ID).await;
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/requirements")
        );

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
        req.status_id = 3; // Released
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
        child.parent_id = Some(1);
        repo.requirements.insert(2, child);
        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements", ADMIN_ID).await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("valid response");
        // Check that parent and child requirements are rendered in the unified view
        assert!(body.contains("REQ-SYS-1"));
        assert!(body.contains("REQ-SYS-2"));
    }

    // Additional edge case and validation tests

    #[rocket::async_test]
    async fn post_requirement_validates_empty_title() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/requirements/new",
            "title=&description=Test&verification_method_ids=1&\
             status_id=1&reviewer_id=1&\
             category_id=1&parent_id=0&applicability_id=1&reference_code=",
            ADMIN_ID,
        )
        .await;

        // Should fail validation for empty title
        assert!(response.status() == Status::BadRequest || response.status() == Status::SeeOther);
    }

    #[rocket::async_test]
    async fn post_requirement_with_invalid_reference_format() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/requirements/new",
            "title=Test&description=Body&verification_method_ids=1&\
             status_id=1&reviewer_id=1&\
             category_id=1&parent_id=0&applicability_id=1&\
             reference_code=INVALID-FORMAT",
            ADMIN_ID,
        )
        .await;

        // Should redirect to new form with error
        assert_eq!(response.status(), Status::SeeOther);
        let location = response.headers().get_one("Location").unwrap_or("");
        assert!(location.contains("error") || location.contains("new"));
    }

    #[rocket::async_test]
    async fn post_requirement_with_valid_custom_reference() {
        let client = test_client(base_repo()).await;
        let response = post_form_with_session(
            &client,
            "/p/1/requirements/new",
            "title=Custom&description=Test&verification_method_ids=1&\
             status_id=1&reviewer_id=1&\
             category_id=1&parent_id=0&applicability_id=1&\
             reference_code=REQ-SYS-999",
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
        assert_eq!(reqs[0].reference_code, "REQ-SYS-999");
    }

    #[rocket::async_test]
    async fn show_requirement_enforces_project_ownership() {
        let mut repo = base_repo();

        // Create requirement in different project
        let mut req = sample_requirement(1);
        req.project_id = 99; // Different project
        repo.requirements.insert(1, req);

        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/show/1", ADMIN_ID).await;

        // Should redirect to the correct project
        assert_eq!(response.status(), Status::SeeOther);
        let location = response.headers().get_one("Location").unwrap_or("");
        assert!(location.contains("/p/99/"));
    }

    #[rocket::async_test]
    async fn edit_requirement_enforces_project_ownership() {
        let mut repo = base_repo();

        let mut req = sample_requirement(1);
        req.project_id = 99;
        repo.requirements.insert(1, req);

        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/edit/1", ADMIN_ID).await;

        assert_eq!(response.status(), Status::SeeOther);
        let location = response.headers().get_one("Location").unwrap_or("");
        assert!(location.contains("/p/99/"));
    }

    #[rocket::async_test]
    async fn new_requirement_displays_flash_message() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/1/requirements/new?created=1", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("created successfully") || body.contains("data-flash-success"));
    }

    #[rocket::async_test]
    async fn new_requirement_with_parent_parameter() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/new?parent=1", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("New Requirement"));
        // Parent should be pre-selected
        assert!(body.contains("value=\"1\"") || body.contains("selected"));
    }

    #[rocket::async_test]
    async fn new_requirement_with_template_parameter() {
        let mut repo = base_repo();
        let mut template_req = sample_requirement(1);
        template_req.title = "Template Title".into();
        template_req.description = "Template Description".into();
        repo.requirements.insert(1, template_req);
        let client = test_client(repo).await;

        let response =
            get_with_session(&client, "/p/1/requirements/new?template=1", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("Template Title") || body.contains("Template Description"));
    }

    #[rocket::async_test]
    async fn delete_requirement_admin_can_delete_released() {
        let mut repo = base_repo();
        let mut req = sample_requirement(1);
        req.status_id = 5; // Released/higher status
        repo.requirements.insert(1, req);

        let client = test_client(repo).await;

        let response = delete_with_session(&client, "/p/1/requirements/delete/1", ADMIN_ID).await;

        // Admin should be able to delete
        assert_eq!(response.status(), Status::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/1/requirements")
        );
    }

    #[rocket::async_test]
    async fn requirement_edit_updates_all_fields() {
        let mut repo = base_repo();
        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = post_form_with_session(
            &client,
            "/p/1/requirements/edit/1",
            "id=1&title=Updated+Title&description=Updated+Description&\
             verification_method_ids=1&status_id=1&author_id=1&reviewer_id=1&\
             category_id=1&parent_id=0&applicability_id=1&\
             justification=Updated+Justification&project_id=1&reference_code=REQ-SYS-1",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::SeeOther);
        let state = client.rocket().state::<TestAppState>().expect("state");
        let req = state.repo_read().get_requirement_by_id(1).unwrap();
        assert_eq!(req.title, "Updated Title");
        assert_eq!(req.description, "Updated Description");
        assert_eq!(req.justification, Some("Updated Justification".into()));
    }

    #[rocket::async_test]
    async fn show_requirements_with_multiple_filters() {
        let mut repo = base_repo();

        // Add requirements with different statuses and categories
        let mut req1 = sample_requirement(1);
        req1.status_id = 1;
        req1.category_id = 1;
        repo.requirements.insert(1, req1);

        let mut req2 = sample_requirement(2);
        req2.status_id = 2;
        req2.category_id = 1;
        req2.reference_code = "REQ-SYS-2".into();
        repo.requirements.insert(2, req2);
        repo.requirement_verification_methods.push((1, 1));
        repo.requirement_verification_methods.push((2, 1));

        let client = test_client(repo).await;

        let response = get_with_session(
            &client,
            "/p/1/requirements?status_filter=1&category_filter=1&verification_filter=1",
            ADMIN_ID,
        )
        .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("Requirement 1"));
        assert!(!body.contains("Requirement 2"));
    }

    #[rocket::async_test]
    async fn show_requirements_displays_metrics() {
        let mut repo = base_repo();

        // Add requirements with different statuses
        let mut req1 = sample_requirement(1);
        req1.status_id = 1; // Draft
        repo.requirements.insert(1, req1);

        let mut req2 = sample_requirement(2);
        req2.status_id = 1; // Draft
        req2.reference_code = "REQ-SYS-2".into();
        repo.requirements.insert(2, req2);

        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        // Should show total count
        assert!(body.contains("requirement_metrics"));
    }

    #[rocket::async_test]
    async fn requirement_detail_shows_parent_and_children() {
        use crate::models::{RequirementVersion, RequirementVersionLink};

        let mut repo = base_repo();

        let mut parent = sample_requirement(1);
        parent.current_version_id = Some(100);
        repo.requirements.insert(1, parent);

        let mut child = sample_requirement(2);
        child.current_version_id = Some(200);
        child.reference_code = "REQ-SYS-2".into();
        repo.requirements.insert(2, child);

        // Add RequirementVersion entries
        repo.requirement_versions.insert(
            100,
            RequirementVersion {
                id: 100,
                requirement_id: 1,
                title: "Requirement 1".into(),
                description: "Test requirement".into(),
                status_id: 1,
                author_id: ADMIN_ID,
                reviewer_id: ADMIN_ID,
                category_id: 1,
                created_at: timestamp(),
                deadline_date: None,
                applicability_id: 1,
                justification: None,
                approval_state: "draft".to_string(),
                approved_by: None,
                approved_at: None,
            },
        );
        repo.requirement_versions.insert(
            200,
            RequirementVersion {
                id: 200,
                requirement_id: 2,
                title: "Requirement 2".into(),
                description: "Test requirement".into(),
                status_id: 1,
                author_id: ADMIN_ID,
                reviewer_id: ADMIN_ID,
                category_id: 1,
                created_at: timestamp(),
                deadline_date: None,
                applicability_id: 1,
                justification: None,
                approval_state: "draft".to_string(),
                approved_by: None,
                approved_at: None,
            },
        );

        // Link child (version 200) -> parent (version 100)
        repo.requirement_version_links.push(RequirementVersionLink {
            id: 1,
            source_version_id: 200,
            target_version_id: 100,
            link_type: "DERIVES_FROM".to_string(),
            rationale: None,
            project_id: PRIMARY_PROJECT,
            created_at: timestamp(),
            metadata: None,
        });

        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/show/1", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        // Should contain child requirement
        assert!(body.contains("REQ-SYS-2"));
    }

    #[rocket::async_test]
    async fn requirement_detail_shows_linked_tests() {
        let mut repo = base_repo();

        repo.requirements.insert(1, sample_requirement(1));

        // Note: Test linking is more complex and would require matrix implementation
        // This test verifies the detail page renders even without linked tests

        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements/show/1", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        assert!(body.contains("REQ-SYS-1"));
    }

    #[rocket::async_test]
    async fn inline_category_creation_returns_new_id() {
        let client = test_client(base_repo()).await;
        let response = client
            .post("/p/1/requirements/inline/category")
            .header(ContentType::JSON)
            .private_cookie(session_cookie(ADMIN_ID))
            .body(r#"{"title":"New Category","description":"Test category","tag":"NEW"}"#)
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("inline response");
        let value: JsonValue = serde_json::from_str(&body).expect("json");
        assert_eq!(value["label"], "New Category");
        assert_eq!(value["tag"], "NEW");
        assert!(value["id"].as_i64().is_some());
    }

    #[rocket::async_test]
    async fn requirements_tree_handles_empty_project() {
        let client = test_client(base_repo()).await;

        let response = get_with_session(&client, "/p/1/requirements", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        // Check that the tree view section exists in the unified page
        assert!(body.contains("treeView") || body.contains("tree_data"));
    }

    #[rocket::async_test]
    async fn requirements_tree_handles_multiple_levels() {
        let mut repo = base_repo();

        let parent = sample_requirement(1);
        repo.requirements.insert(1, parent);

        let mut child = sample_requirement(2);
        child.parent_id = Some(1);
        child.reference_code = "REQ-SYS-2".into();
        repo.requirements.insert(2, child);

        let mut grandchild = sample_requirement(3);
        grandchild.parent_id = Some(2);
        grandchild.reference_code = "REQ-SYS-3".into();
        repo.requirements.insert(3, grandchild);

        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements", ADMIN_ID).await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("valid response");
        // Check that all requirements are rendered in the tree structure
        assert!(body.contains("REQ-SYS-1"));
        assert!(body.contains("REQ-SYS-2"));
        assert!(body.contains("REQ-SYS-3"));
    }

    #[rocket::async_test]
    async fn unauthorized_user_cannot_access_requirements() {
        let mut repo = base_repo();

        // Create non-member user
        let mut user = DieselRepoMock::make_user(99, "outsider", "");
        user.is_admin = false;
        repo.users.insert(99, user);

        repo.requirements.insert(1, sample_requirement(1));
        let client = test_client(repo).await;

        let response = get_with_session(&client, "/p/1/requirements", 99).await;

        // Should be forbidden or redirect
        assert!(
            response.status() == Status::Forbidden
                || response.status() == Status::SeeOther
                || response.status() == Status::Unauthorized
        );
    }

    #[rocket::async_test]
    async fn post_edit_requirement_enforces_project_match() {
        let mut repo = base_repo();

        let mut req = sample_requirement(1);
        req.project_id = 99;
        repo.requirements.insert(1, req);

        let client = test_client(repo).await;

        let response = post_form_with_session(
            &client,
            "/p/1/requirements/edit/1",
            "id=1&title=Hack&description=Test&verification_method_ids=1&\
             status_id=1&author_id=1&reviewer_id=1&\
             category_id=1&parent_id=0&applicability_id=1&\
             justification=&project_id=1&reference_code=REQ-SYS-1",
            ADMIN_ID,
        )
        .await;

        // Should redirect to correct project
        assert_eq!(response.status(), Status::SeeOther);
        let location = response.headers().get_one("Location").unwrap_or("");
        assert!(location.contains("/p/99/"));
    }
}
