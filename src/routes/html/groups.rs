// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#![allow(clippy::result_large_err)]

use std::collections::{HashMap, HashSet};

use super::helpers::*;
use super::prelude::*;
use crate::permissions::{group_role_label, has_group_permission, GroupPermission};
use crate::repository::errors::RepoError;
use crate::repository::{
    GroupMembersRepository, GroupsRepository, ProjectMembersRepository, UserRepository,
};
use crate::services::GroupService;
use rocket::serde::json::Value;

#[derive(FromForm)]
struct GroupMemberForm {
    user_id: i32,
    role: i32,
}

#[derive(FromForm)]
struct GroupRoleForm {
    role: i32,
}

fn group_role_options() -> Vec<Value> {
    vec![
        json!({ "id": 1, "label": group_role_label(1) }),
        json!({ "id": 2, "label": group_role_label(2) }),
        json!({ "id": 3, "label": group_role_label(3) }),
        json!({ "id": 4, "label": group_role_label(4) }),
    ]
}

fn friendly_group_error(error: RepoError, fallback: &str) -> String {
    match error {
        RepoError::BadInput(message) | RepoError::Duplicate(message) => message,
        RepoError::NotFound => "The requested group could not be found.".to_string(),
        _ => fallback.to_string(),
    }
}

fn current_group_role_label(
    repo: &crate::app::DieselCachedRepo,
    user: &User,
    group_id: i32,
) -> String {
    if user.is_admin {
        return "Administrator".to_string();
    }

    repo.get_groups_for_user(user.id)
        .ok()
        .and_then(|memberships| {
            memberships
                .into_iter()
                .find(|membership| membership.group_id == group_id)
        })
        .map(|membership| group_role_label(membership.role).to_string())
        .unwrap_or_else(|| "Member".to_string())
}

fn decorate_groups_for_listing(
    state: &State<AppState>,
    user: &User,
    groups: &[Group],
) -> Vec<Value> {
    let repo = state.repo_read();
    let users = repo.get_users_all().unwrap_or_default();
    let user_lookup: HashMap<i32, String> =
        users.into_iter().map(|item| (item.id, item.name)).collect();
    let group_memberships: HashMap<i32, i32> = if user.is_admin {
        HashMap::new()
    } else {
        repo.get_groups_for_user(user.id)
            .unwrap_or_default()
            .into_iter()
            .map(|membership| (membership.group_id, membership.role))
            .collect()
    };

    groups
        .iter()
        .map(|group| {
            let members = repo.get_members_by_group(group.id).unwrap_or_default();
            let projects = repo.get_projects_by_group(group.id).unwrap_or_default();
            let owners: Vec<String> = members
                .iter()
                .filter(|member| member.role == 1)
                .filter_map(|member| user_lookup.get(&member.user_id).cloned())
                .collect();
            let owners_label = if owners.is_empty() {
                "No owners assigned".to_string()
            } else {
                owners.join(", ")
            };

            let current_role = group_memberships
                .get(&group.id)
                .map(|role| group_role_label(*role).to_string())
                .unwrap_or_else(|| {
                    if user.is_admin {
                        "Administrator".to_string()
                    } else {
                        "Member".to_string()
                    }
                });

            json!({
                "id": group.id,
                "name": group.name,
                "slug": group.slug,
                "description": group.description,
                "owners": owners,
                "owners_label": owners_label,
                "member_count": members.len(),
                "project_count": projects.len(),
                "current_role_label": current_role,
                "updated_at": group.updated_at.format("%Y-%m-%d").to_string(),
            })
        })
        .collect()
}

fn build_group_members_page_context(
    state: &State<AppState>,
    user: User,
    group_id: i32,
    group_slug: &str,
    cookies: &CookieJar<'_>,
    error: Option<String>,
    success: Option<String>,
) -> Result<Value, Redirect> {
    let mut ctx = build_context_with_projects(state, user.clone(), cookies);
    let repo = state.repo_read();
    let group = repo.get_group_by_id(group_id).map_err(|_| {
        Redirect::to(uri!(show_groups(
            success = Option::<String>::None,
            error = Option::<String>::None
        )))
    })?;
    let members = repo.get_members_by_group(group_id).unwrap_or_default();
    let users = repo.get_users_all().unwrap_or_default();
    let owner_count = members.iter().filter(|member| member.role == 1).count();
    let can_manage_members =
        has_group_permission(&*repo, &user, group_id, GroupPermission::ManageGroupMembers);

    let user_lookup: HashMap<i32, &User> = users.iter().map(|member| (member.id, member)).collect();
    let decorated_members: Vec<Value> = members
        .iter()
        .map(|membership| {
            let (name, username, email, is_admin) = user_lookup
                .get(&membership.user_id)
                .map(|member| {
                    (
                        member.name.clone(),
                        member.username.clone(),
                        member.email.clone(),
                        member.is_admin,
                    )
                })
                .unwrap_or_else(|| {
                    (
                        format!("Unknown User #{}", membership.user_id),
                        "unknown".to_string(),
                        String::new(),
                        false,
                    )
                });

            let is_last_owner = membership.role == 1 && owner_count <= 1;

            json!({
                "id": membership.user_id,
                "name": name,
                "username": username,
                "email": email,
                "role_id": membership.role,
                "role_label": group_role_label(membership.role),
                "is_admin": is_admin,
                "can_remove": can_manage_members && !is_last_owner,
                "can_change_role": can_manage_members && !is_last_owner,
                "is_last_owner": is_last_owner,
            })
        })
        .collect();

    let member_ids: HashSet<i32> = members
        .iter()
        .map(|membership| membership.user_id)
        .collect();
    let available_users: Vec<Value> = if can_manage_members {
        users
            .iter()
            .filter(|candidate| !member_ids.contains(&candidate.id))
            .map(|candidate| {
                json!({
                    "id": candidate.id,
                    "label": format!("{} (@{})", candidate.name, candidate.username),
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    let has_available_users = !available_users.is_empty();
    let group_name = group.name.clone();
    let group_description = group.description.clone();

    if let Some(ctx_obj) = ctx.as_object_mut() {
        ctx_obj.insert(
            "group".to_string(),
            json!({
                "id": group.id,
                "name": group_name.clone(),
                "slug": group.slug,
                "description": group_description,
            }),
        );
        ctx_obj.insert("members".to_string(), json!(decorated_members));
        ctx_obj.insert("member_count".to_string(), json!(members.len()));
        ctx_obj.insert("owner_count".to_string(), json!(owner_count));
        ctx_obj.insert("can_manage_members".to_string(), json!(can_manage_members));
        ctx_obj.insert("available_users".to_string(), json!(available_users));
        ctx_obj.insert(
            "has_available_users".to_string(),
            json!(has_available_users),
        );
        ctx_obj.insert("role_options".to_string(), json!(group_role_options()));
        ctx_obj.insert("group_slug".to_string(), json!(group_slug));
        ctx_obj.insert(
            "page_title".to_string(),
            json!(format!("{} - Members", group_name)),
        );
        ctx_obj.insert("error".to_string(), json!(error));
        ctx_obj.insert("success".to_string(), json!(success));
    }

    Ok(ctx)
}

#[get("/groups?<success>&<error>")]
fn show_groups(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
    success: Option<String>,
    error: Option<String>,
) -> Template {
    let user = session_user.into_inner();
    let groups = get_accessible_groups(state, &user);
    let decorated_groups = decorate_groups_for_listing(state, &user, &groups);
    let mut ctx = build_context_with_projects(state, user, cookies);

    if let Some(ctx_obj) = ctx.as_object_mut() {
        ctx_obj.insert("groups".to_string(), json!(decorated_groups));
        ctx_obj.insert("success".to_string(), json!(success));
        ctx_obj.insert("error".to_string(), json!(error));
        ctx_obj.insert("page_title".to_string(), json!("Groups"));
    }

    Template::render("groups", ctx)
}

#[get("/groups/new?<error>")]
fn new_group(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
    error: Option<String>,
) -> Template {
    let user = session_user.into_inner();
    let mut ctx = build_context_with_projects(state, user, cookies);
    let form = json!({
        "name": "",
        "description": "",
    });

    if let Some(ctx_obj) = ctx.as_object_mut() {
        ctx_obj.insert("form".to_string(), form);
        ctx_obj.insert("error".to_string(), json!(error));
        ctx_obj.insert("page_title".to_string(), json!("New Group"));
    }

    Template::render("new_group", ctx)
}

#[post("/groups", data = "<new_group_form>")]
fn post_group(
    session_user: SessionUser,
    new_group_form: Form<NewGroup>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let user = session_user.into_inner();
    let service = GroupService::new(state);
    let submitted = new_group_form.into_inner();

    match service.create(&user, submitted) {
        Ok(group_id) => match service.get_by_id(group_id) {
            Ok(group) => Ok(Redirect::to(uri!(show_group(
                group_slug = group.slug,
                success = Some("Group created successfully.".to_string()),
                error = Option::<String>::None
            )))),
            Err(_) => Ok(Redirect::to(uri!(show_groups(
                success = Some("Group created successfully.".to_string()),
                error = Option::<String>::None
            )))),
        },
        Err(error) => Err(Redirect::to(uri!(new_group(
            error = Some(friendly_group_error(
                error,
                "Failed to create group. Please try again."
            ))
        )))),
    }
}

#[get("/g/<group_slug>?<success>&<error>")]
fn show_group(
    group_access: HtmlGroupAccess,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
    group_slug: &str,
    success: Option<String>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    let (user, group_id, resolved_slug) = group_access.into_parts();
    let mut ctx = build_context_with_projects(state, user.clone(), cookies);
    let repo = state.repo_read();
    let group = repo.get_group_by_id(group_id).map_err(|_| {
        Redirect::to(uri!(show_groups(
            success = Option::<String>::None,
            error = Option::<String>::None
        )))
    })?;
    let members = repo.get_members_by_group(group_id).unwrap_or_default();
    let projects = repo.get_projects_by_group(group_id).unwrap_or_default();
    let users = repo.get_users_all().unwrap_or_default();
    let user_lookup: HashMap<i32, &User> = users.iter().map(|item| (item.id, item)).collect();
    let accessible_project_ids: HashSet<i32> = if user.is_admin {
        projects.iter().map(|project| project.id).collect()
    } else {
        repo.get_projects_for_user(user.id)
            .unwrap_or_default()
            .into_iter()
            .map(|membership| membership.project_id)
            .collect()
    };
    let owners: Vec<String> = members
        .iter()
        .filter(|member| member.role == 1)
        .filter_map(|member| {
            user_lookup
                .get(&member.user_id)
                .map(|user| user.name.clone())
        })
        .collect();
    let owners_label = if owners.is_empty() {
        "No owners assigned".to_string()
    } else {
        owners.join(", ")
    };
    let current_role_label = current_group_role_label(&repo, &user, group_id);
    let can_manage_members =
        has_group_permission(&*repo, &user, group_id, GroupPermission::ManageGroupMembers);

    let mut related_projects: Vec<Value> = projects
        .into_iter()
        .map(|project| {
            let can_open = accessible_project_ids.contains(&project.id);
            let owner_name = project
                .owner_id
                .and_then(|owner_id| user_lookup.get(&owner_id).map(|owner| owner.name.clone()));

            json!({
                "id": project.id,
                "slug": project.slug,
                "name": project.name,
                "description": project.description,
                "status_label": project.status.title(),
                "owner_name": owner_name,
                "can_open": can_open,
                "can_edit_project": user.is_admin,
                "restricted_message": if can_open {
                    Option::<String>::None
                } else {
                    Some("You can see that this project belongs to the group, but you do not have direct access to open it.".to_string())
                },
            })
        })
        .collect();
    related_projects.sort_by(|left, right| {
        let left_name = left
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let right_name = right
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        left_name.to_lowercase().cmp(&right_name.to_lowercase())
    });

    if let Some(ctx_obj) = ctx.as_object_mut() {
        ctx_obj.insert(
            "group".to_string(),
            json!({
                "id": group.id,
                "name": group.name.clone(),
                "slug": resolved_slug,
                "description": group.description.clone(),
                "member_count": members.len(),
                "project_count": related_projects.len(),
                "owners": owners,
                "owners_label": owners_label,
                "current_role_label": current_role_label,
            }),
        );
        ctx_obj.insert("related_projects".to_string(), json!(related_projects));
        ctx_obj.insert("can_manage_members".to_string(), json!(can_manage_members));
        ctx_obj.insert(
            "page_title".to_string(),
            json!(format!("{} - Group", group.name)),
        );
        ctx_obj.insert("success".to_string(), json!(success));
        ctx_obj.insert("error".to_string(), json!(error));
        ctx_obj.insert("group_slug".to_string(), json!(group_slug));
    }

    Ok(Template::render("group", ctx))
}

#[get("/g/<group_slug>/members?<success>&<error>")]
fn show_group_members(
    group_access: HtmlGroupAccess,
    group_slug: &str,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
    success: Option<String>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    let (user, group_id, _) = group_access.into_parts();
    let ctx = build_group_members_page_context(
        state,
        user,
        group_id,
        &group_slug,
        cookies,
        error,
        success,
    )?;

    Ok(Template::render("group_members", ctx))
}

#[get("/g/<group_slug>/edit?<success>&<error>")]
fn edit_group(
    manage_access: HtmlGroupManageAccess,
    group_slug: &str,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
    success: Option<String>,
    error: Option<String>,
) -> Result<Template, Redirect> {
    let (user, group_id, _) = manage_access.into_parts();
    let mut ctx = build_context_with_projects(state, user, cookies);
    let repo = state.repo_read();
    let group = repo.get_group_by_id(group_id).map_err(|_| {
        Redirect::to(uri!(show_groups(
            success = Option::<String>::None,
            error = Option::<String>::None
        )))
    })?;
    let project_count = repo
        .get_projects_by_group(group_id)
        .map(|items| items.len())
        .unwrap_or(0);
    let member_count = repo
        .get_members_by_group(group_id)
        .map(|items| items.len())
        .unwrap_or(0);

    if let Some(ctx_obj) = ctx.as_object_mut() {
        ctx_obj.insert(
            "group".to_string(),
            json!({
                "id": group.id,
                "name": group.name.clone(),
                "slug": group_slug,
                "description": group.description.clone(),
                "project_count": project_count,
                "member_count": member_count,
            }),
        );
        ctx_obj.insert("success".to_string(), json!(success));
        ctx_obj.insert("error".to_string(), json!(error));
        ctx_obj.insert(
            "page_title".to_string(),
            json!(format!("Edit {} - Group", group.name)),
        );
    }

    Ok(Template::render("edit_group", ctx))
}

#[post("/g/<group_slug>/edit", data = "<group_form>")]
fn post_edit_group(
    manage_access: HtmlGroupManageAccess,
    group_slug: &str,
    group_form: Form<UpdateGroup>,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let (user, group_id, _) = manage_access.into_parts();
    let mut payload = group_form.into_inner();
    payload.owner_id = None;

    let service = GroupService::new(state);
    match service.update(&user, group_id, payload) {
        Ok(_) => Ok(Redirect::to(uri!(show_group(
            group_slug = group_slug,
            success = Some("Group updated successfully.".to_string()),
            error = Option::<String>::None
        )))),
        Err(error) => Err(Redirect::to(uri!(edit_group(
            group_slug = group_slug,
            success = Option::<String>::None,
            error = Some(friendly_group_error(
                error,
                "Failed to update group. Please try again."
            ))
        )))),
    }
}

#[post("/g/<group_slug>/delete")]
fn delete_group(
    manage_access: HtmlGroupManageAccess,
    group_slug: &str,
    state: &State<AppState>,
) -> Result<Redirect, Redirect> {
    let (user, group_id, _) = manage_access.into_parts();
    let service = GroupService::new(state);

    match service.delete(&user, group_id) {
        Ok(_) => Ok(Redirect::to(uri!(show_groups(
            success = Some("Group deleted successfully.".to_string()),
            error = Option::<String>::None
        )))),
        Err(error) => Err(Redirect::to(uri!(edit_group(
            group_slug = group_slug,
            success = Option::<String>::None,
            error = Some(friendly_group_error(
                error,
                "Failed to delete group. Please try again."
            ))
        )))),
    }
}

#[post("/g/<group_slug>/members", data = "<member_form>")]
fn add_group_member(
    manage_access: HtmlGroupManageAccess,
    group_slug: &str,
    member_form: Form<GroupMemberForm>,
    state: &State<AppState>,
) -> Redirect {
    let (_user, group_id, _) = manage_access.into_parts();
    let payload = member_form.into_inner();
    let service = GroupService::new(state);

    match service.set_member_role(group_id, payload.user_id, payload.role) {
        Ok(_) => Redirect::to(uri!(show_group_members(
            group_slug = group_slug,
            success = Some("Member added successfully.".to_string()),
            error = Option::<String>::None
        ))),
        Err(error) => Redirect::to(uri!(show_group_members(
            group_slug = group_slug,
            success = Option::<String>::None,
            error = Some(friendly_group_error(
                error,
                "Failed to add member. Please try again."
            ))
        ))),
    }
}

#[post("/g/<group_slug>/members/<member_id>/role", data = "<role_form>")]
fn update_group_member_role(
    manage_access: HtmlGroupManageAccess,
    group_slug: &str,
    member_id: i32,
    role_form: Form<GroupRoleForm>,
    state: &State<AppState>,
) -> Redirect {
    let (_user, group_id, _) = manage_access.into_parts();
    let payload = role_form.into_inner();
    let service = GroupService::new(state);

    match service.set_member_role(group_id, member_id, payload.role) {
        Ok(_) => Redirect::to(uri!(show_group_members(
            group_slug = group_slug,
            success = Some("Member role updated successfully.".to_string()),
            error = Option::<String>::None
        ))),
        Err(error) => Redirect::to(uri!(show_group_members(
            group_slug = group_slug,
            success = Option::<String>::None,
            error = Some(friendly_group_error(
                error,
                "Failed to update member role. Please try again."
            ))
        ))),
    }
}

#[post("/g/<group_slug>/members/<member_id>/remove")]
fn remove_group_member(
    manage_access: HtmlGroupManageAccess,
    group_slug: &str,
    member_id: i32,
    state: &State<AppState>,
) -> Redirect {
    let (_user, group_id, _) = manage_access.into_parts();
    let service = GroupService::new(state);

    match service.remove_member(group_id, member_id) {
        Ok(_) => Redirect::to(uri!(show_group_members(
            group_slug = group_slug,
            success = Some("Member removed successfully.".to_string()),
            error = Option::<String>::None
        ))),
        Err(error) => Redirect::to(uri!(show_group_members(
            group_slug = group_slug,
            success = Option::<String>::None,
            error = Some(friendly_group_error(
                error,
                "Failed to remove member. Please try again."
            ))
        ))),
    }
}

pub fn routes() -> Vec<Route> {
    routes![
        show_groups,
        new_group,
        post_group,
        show_group,
        show_group_members,
        edit_group,
        post_edit_group,
        delete_group,
        add_group_member,
        update_group_member_role,
        remove_group_member
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::auth::session::SESSION_COOKIE;
    use crate::models::{GroupMember, Project, ProjectMember};
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use crate::status_enums::ProjectStatus;
    use chrono::{NaiveDate, NaiveDateTime};
    use rocket::http::{ContentType, Cookie, SameSite, Status};
    use rocket::local::asynchronous::Client;
    use rocket::response::content;
    use rocket::Request;
    use rocket_dyn_templates::Template;
    use std::sync::{Arc, RwLock};

    type TestAppState = AppState<CacheRepository<DieselRepoMock>>;

    const ADMIN_ID: i32 = 1;
    const OWNER_ID: i32 = 2;
    const MEMBER_ID: i32 = 3;
    const OUTSIDER_ID: i32 = 4;

    fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    }

    fn managed_state(repo: DieselRepoMock) -> TestAppState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn test_client(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(managed_state(repo))
            .attach(Template::fairing())
            .mount("/", routes())
            .register("/", catchers![forbidden_catcher]);

        Client::tracked(rocket).await.expect("client")
    }

    fn session_cookie(id: i32) -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, id.to_string());
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.set_same_site(SameSite::Strict);
        cookie
    }

    fn admin_user() -> User {
        let mut user = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
        user.is_admin = true;
        user.name = "Admin User".into();
        user.email = "admin@example.com".into();
        user
    }

    fn named_user(id: i32, username: &str, name: &str) -> User {
        let mut user = DieselRepoMock::make_user(id, username, "");
        user.name = name.into();
        user.email = format!("{username}@example.com");
        user
    }

    fn group(id: i32, name: &str) -> Group {
        Group {
            id,
            name: name.into(),
            slug: name.to_lowercase().replace(' ', "-"),
            description: Some(format!("{name} description")),
            owner_id: Some(OWNER_ID),
            created_at: timestamp(),
            updated_at: timestamp(),
        }
    }

    fn project(id: i32, name: &str, slug: &str, group_id: Option<i32>) -> Project {
        Project {
            id,
            name: name.into(),
            description: Some(format!("{name} description")),
            creation_date: Some(timestamp()),
            update_date: Some(timestamp()),
            status: ProjectStatus::Active,
            owner_id: Some(OWNER_ID),
            slug: slug.into(),
            group_id,
        }
    }

    #[catch(403)]
    fn forbidden_catcher(_req: &Request) -> content::RawHtml<&'static str> {
        content::RawHtml("Access Denied")
    }

    fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(ADMIN_ID, admin_user());
        repo.users
            .insert(OWNER_ID, named_user(OWNER_ID, "owner", "Group Owner"));
        repo.users
            .insert(MEMBER_ID, named_user(MEMBER_ID, "member", "Group Member"));
        repo.users.insert(
            OUTSIDER_ID,
            named_user(OUTSIDER_ID, "outsider", "Outside User"),
        );

        repo.groups.insert(11, group(11, "Flight Systems"));
        repo.groups.insert(12, group(12, "Payload"));

        repo.group_members.push(GroupMember {
            group_id: 11,
            user_id: OWNER_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo.group_members.push(GroupMember {
            group_id: 11,
            user_id: MEMBER_ID,
            role: 3,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo.group_members.push(GroupMember {
            group_id: 12,
            user_id: OWNER_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        repo.projects
            .insert(21, project(21, "Flight Deck", "flight-deck", Some(11)));
        repo.projects
            .insert(22, project(22, "Telemetry", "telemetry", Some(11)));

        repo.project_members.push(ProjectMember {
            project_id: 21,
            user_id: OWNER_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo.project_members.push(ProjectMember {
            project_id: 21,
            user_id: MEMBER_ID,
            role: 2,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo.project_members.push(ProjectMember {
            project_id: 22,
            user_id: OWNER_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });

        repo
    }

    #[rocket::async_test]
    async fn groups_page_lists_only_accessible_groups() {
        let client = test_client(base_repo()).await;
        let response = client
            .get("/groups")
            .private_cookie(session_cookie(MEMBER_ID))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Flight Systems"));
        assert!(!body.contains("Payload"));
    }

    #[rocket::async_test]
    async fn any_authenticated_user_can_create_group() {
        let client = test_client(base_repo()).await;
        let response = client
            .post("/groups")
            .private_cookie(session_cookie(MEMBER_ID))
            .header(ContentType::Form)
            .body("name=New+Org&description=Created+from+html")
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::SeeOther);
        let location = response.headers().get_one("Location").unwrap_or_default();
        assert!(location.contains("/g/new-org"));
    }

    #[rocket::async_test]
    async fn non_member_cannot_view_group_pages() {
        let client = test_client(base_repo()).await;
        let detail = client
            .get("/g/flight-systems")
            .private_cookie(session_cookie(OUTSIDER_ID))
            .dispatch()
            .await;
        assert_eq!(detail.status(), Status::Forbidden);

        let members = client
            .get("/g/flight-systems/members")
            .private_cookie(session_cookie(OUTSIDER_ID))
            .dispatch()
            .await;
        assert_eq!(members.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn owner_can_manage_members_and_non_owner_cannot_open_settings() {
        let client = test_client(base_repo()).await;

        let add_response = client
            .post("/g/flight-systems/members")
            .private_cookie(session_cookie(OWNER_ID))
            .header(ContentType::Form)
            .body(format!("user_id={OUTSIDER_ID}&role=4"))
            .dispatch()
            .await;
        assert_eq!(add_response.status(), Status::SeeOther);

        let settings_response = client
            .get("/g/flight-systems/edit")
            .private_cookie(session_cookie(MEMBER_ID))
            .dispatch()
            .await;
        assert_eq!(settings_response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    async fn last_owner_demotion_is_blocked_until_another_owner_exists() {
        let client = test_client(base_repo()).await;

        let blocked = client
            .post("/g/flight-systems/members/2/role")
            .private_cookie(session_cookie(OWNER_ID))
            .header(ContentType::Form)
            .body("role=2")
            .dispatch()
            .await;
        assert_eq!(blocked.status(), Status::SeeOther);
        let blocked_location = blocked.headers().get_one("Location").unwrap_or_default();
        assert!(blocked_location.contains("error="));

        let promote = client
            .post("/g/flight-systems/members/3/role")
            .private_cookie(session_cookie(OWNER_ID))
            .header(ContentType::Form)
            .body("role=1")
            .dispatch()
            .await;
        assert_eq!(promote.status(), Status::SeeOther);

        let demote = client
            .post("/g/flight-systems/members/2/role")
            .private_cookie(session_cookie(OWNER_ID))
            .header(ContentType::Form)
            .body("role=2")
            .dispatch()
            .await;
        assert_eq!(demote.status(), Status::SeeOther);
        let demote_location = demote.headers().get_one("Location").unwrap_or_default();
        assert!(demote_location.contains("success="));
    }

    #[rocket::async_test]
    async fn group_detail_lists_restricted_projects_without_open_links() {
        let client = test_client(base_repo()).await;
        let response = client
            .get("/g/flight-systems")
            .private_cookie(session_cookie(MEMBER_ID))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Flight Deck"));
        assert!(body.contains("Telemetry"));
        assert!(body.contains("/p/flight-deck"));
        assert!(body.contains("do not have direct access"));
    }

    #[rocket::async_test]
    async fn deleting_group_with_projects_is_rejected() {
        let client = test_client(base_repo()).await;
        let response = client
            .post("/g/flight-systems/delete")
            .private_cookie(session_cookie(OWNER_ID))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::SeeOther);
        let location = response.headers().get_one("Location").unwrap_or_default();
        assert!(location.contains("/g/flight-systems/edit"));
        assert!(location.contains("error="));
    }
}
