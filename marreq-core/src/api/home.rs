// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! The start screen after sign-in (issue #387): what needs the user's
//! attention across their projects, and a quick search over requirements.
//!
//! Both routes use the browser session only: they span every project the
//! user can see, which a project-scoped API token must not.

use chrono::NaiveDateTime;
use rocket::serde::Serialize;

use crate::api::prelude::*;
use crate::auth::guards::SessionUser;
use crate::models::{Project, User};
use crate::permissions::{has_permission, may_change_review_gates};
use crate::repository::{MatrixRepository, NotificationRepository, RequirementsRepository};
use crate::services::ProjectService;

/// Most attention items returned; the start screen shows the first few.
const MAX_ATTENTION_ITEMS: usize = 50;
/// Unread notifications considered for the attention list.
const NOTIFICATION_SCAN: i64 = 100;
/// Notification types that ask something of the user. `approval_requested`
/// is left out: pending approvals are computed live instead.
const ATTENTION_NOTIFICATIONS: [&str; 3] = ["review_assigned", "comment_added", "mentioned"];
const DEFAULT_SEARCH_LIMIT: usize = 20;
const MAX_SEARCH_LIMIT: usize = 50;

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AttentionKind {
    /// A requirement version marked reviewed, waiting for the user's approval.
    Approval,
    /// Draft requirements in a project where the user is a reviewer (one row per project).
    Review,
    /// Suspect traceability links in a project the user can edit (one row per project).
    Suspect,
    /// An unread notification that asks something of the user.
    Notification,
}

#[derive(Debug, Serialize)]
pub struct AttentionItem {
    pub kind: AttentionKind,
    pub project_id: i32,
    pub title: String,
    pub requirement_id: Option<i32>,
    pub reference_code: Option<String>,
    /// Number of drafts or suspect links for the aggregated kinds.
    pub count: Option<usize>,
    pub notification_id: Option<i32>,
    pub notification_type: Option<String>,
    /// When the item last changed; the list is sorted on it, newest first.
    pub at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize)]
pub struct AttentionResponse {
    pub items: Vec<AttentionItem>,
    pub approvals: usize,
    pub reviews: usize,
    pub suspect_links: usize,
    pub notifications: usize,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub project_id: i32,
    pub requirement_id: i32,
    pub reference_code: String,
    pub title: String,
}

/// The projects listed for the user, as on `/api/dashboard`.
fn visible_projects(state: &AppState, user: &User) -> Vec<Project> {
    let service = ProjectService::new(state);
    if user.is_admin {
        service.list_all().unwrap_or_default()
    } else {
        service.get_by_user_id(user.id).unwrap_or_default()
    }
}

/// GET /api/home/attention — what needs the user's attention, newest first.
#[get("/home/attention")]
pub fn attention(user: SessionUser, state: &State<AppState>) -> ApiResult<Json<AttentionResponse>> {
    let user = user.into_inner();
    Ok(Json(collect_attention(state.inner(), &user)?))
}

pub fn collect_attention(state: &AppState, user: &User) -> ApiResult<AttentionResponse> {
    let projects = visible_projects(state, user);
    let repo = state.repo_read();
    let mut items = Vec::new();
    let (mut approvals, mut reviews, mut suspect_links) = (0, 0, 0);

    for project in projects.iter().filter(|p| !p.is_archived()) {
        // Archived projects are read-only, so nothing in them can be acted on.
        if may_change_review_gates(&*repo, user, project.id) {
            let requirements = repo.get_requirements_by_project(project.id)?;
            let mut drafts = 0;
            let mut latest_draft: Option<NaiveDateTime> = None;
            for requirement in &requirements {
                match requirement.approval_state.to_lowercase().as_str() {
                    "reviewed" => {
                        approvals += 1;
                        items.push(AttentionItem {
                            kind: AttentionKind::Approval,
                            project_id: project.id,
                            title: format!(
                                "{} is waiting for your approval",
                                requirement.reference_code
                            ),
                            requirement_id: Some(requirement.id),
                            reference_code: Some(requirement.reference_code.clone()),
                            count: None,
                            notification_id: None,
                            notification_type: None,
                            at: Some(requirement.update_date),
                        });
                    }
                    "draft" => {
                        drafts += 1;
                        latest_draft = latest_draft.max(Some(requirement.update_date));
                    }
                    _ => {}
                }
            }
            if drafts > 0 {
                reviews += drafts;
                items.push(AttentionItem {
                    kind: AttentionKind::Review,
                    project_id: project.id,
                    title: format!(
                        "{drafts} draft requirement{} to review",
                        if drafts == 1 { "" } else { "s" }
                    ),
                    requirement_id: None,
                    reference_code: None,
                    count: Some(drafts),
                    notification_id: None,
                    notification_type: None,
                    at: latest_draft,
                });
            }
        }

        if has_permission(&*repo, user, project.id, Permission::EditRequirements) {
            let suspect: Vec<_> = repo
                .get_matrix_by_project(project.id)?
                .into_iter()
                .filter(|link| link.suspect)
                .collect();
            if !suspect.is_empty() {
                suspect_links += suspect.len();
                items.push(AttentionItem {
                    kind: AttentionKind::Suspect,
                    project_id: project.id,
                    title: format!(
                        "{} suspect link{} to review",
                        suspect.len(),
                        if suspect.len() == 1 { "" } else { "s" }
                    ),
                    requirement_id: None,
                    reference_code: None,
                    count: Some(suspect.len()),
                    notification_id: None,
                    notification_type: None,
                    at: suspect.iter().filter_map(|link| link.suspect_at).max(),
                });
            }
        }
    }

    let listed: Vec<i32> = projects.iter().map(|p| p.id).collect();
    let mut notifications = 0;
    for n in repo.get_notifications_for_user(user.id, NOTIFICATION_SCAN, true)? {
        let Some(project_id) = n.project_id.filter(|id| listed.contains(id)) else {
            continue;
        };
        if !ATTENTION_NOTIFICATIONS.contains(&n.notification_type.as_str()) {
            continue;
        }
        notifications += 1;
        items.push(AttentionItem {
            kind: AttentionKind::Notification,
            project_id,
            title: n.title,
            requirement_id: n
                .entity_id
                .filter(|_| n.entity_type.as_deref() == Some("requirement")),
            reference_code: None,
            count: None,
            notification_id: Some(n.id),
            notification_type: Some(n.notification_type),
            at: Some(n.created_at),
        });
    }

    items.sort_by_key(|item| std::cmp::Reverse(item.at));
    items.truncate(MAX_ATTENTION_ITEMS);
    Ok(AttentionResponse {
        items,
        approvals,
        reviews,
        suspect_links,
        notifications,
    })
}

/// GET /api/search?q=&limit= — requirements whose reference code or title
/// contains `q`, across the projects the user can see. Exact and prefix
/// matches on the reference code come first.
#[get("/search?<q>&<limit>")]
pub fn search(
    user: SessionUser,
    state: &State<AppState>,
    q: &str,
    limit: Option<usize>,
) -> ApiResult<Json<Vec<SearchHit>>> {
    let user = user.into_inner();
    let limit = limit
        .unwrap_or(DEFAULT_SEARCH_LIMIT)
        .clamp(1, MAX_SEARCH_LIMIT);
    Ok(Json(search_requirements(state.inner(), &user, q, limit)?))
}

pub fn search_requirements(
    state: &AppState,
    user: &User,
    q: &str,
    limit: usize,
) -> ApiResult<Vec<SearchHit>> {
    let needle = q.trim().to_lowercase();
    if needle.chars().count() < 2 {
        return Ok(Vec::new());
    }
    let projects = visible_projects(state, user);
    let repo = state.repo_read();
    let mut scored = Vec::new();
    for project in &projects {
        if !has_permission(&*repo, user, project.id, Permission::ViewRequirements) {
            continue;
        }
        for requirement in repo.get_requirements_by_project(project.id)? {
            let code = requirement.reference_code.to_lowercase();
            let score = if code == needle {
                0
            } else if code.starts_with(&needle) {
                1
            } else if code.contains(&needle) {
                2
            } else if requirement.title.to_lowercase().contains(&needle) {
                3
            } else {
                continue;
            };
            scored.push((
                score,
                SearchHit {
                    project_id: project.id,
                    requirement_id: requirement.id,
                    reference_code: requirement.reference_code,
                    title: requirement.title,
                },
            ));
        }
    }
    scored.sort_by(|(a, x), (b, y)| {
        a.cmp(b)
            .then_with(|| x.reference_code.cmp(&y.reference_code))
    });
    Ok(scored.into_iter().take(limit).map(|(_, hit)| hit).collect())
}
