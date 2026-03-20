// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::helpers::*;
use super::prelude::*;
use crate::permissions::{has_permission, Permission};
use rocket::serde::json::Value;

#[derive(FromForm)]
pub struct ProjectMemberForm {
    pub id: i32,
    pub role: i32,
}

fn can_manage_members(
    state: &State<AppState>,
    user: &crate::models::User,
    project_id: i32,
) -> bool {
    let repo = state.repo_read();
    has_permission(&*repo, user, project_id, Permission::ManageProjectMembers)
}

fn can_remove_member(
    can_manage_members: bool,
    owner_count: usize,
    member: &ProjectMember,
    current_user_id: i32,
) -> bool {
    if !can_manage_members {
        return false;
    }

    let is_owner = member.role == 1;
    let is_last_owner = is_owner && owner_count <= 1;
    if is_last_owner {
        return false;
    }

    if member.user_id == current_user_id && is_owner && owner_count <= 1 {
        return false;
    }

    true
}

#[get("/<project_id>/members")]
async fn show_project_members(
    project_access: HtmlProjectAccess,
    project_id: String,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();
    cookies.add(Cookie::new("selected_project_id", project_id.to_string()));

    let mut ctx = build_context_with_projects(state, user.clone(), cookies);

    let repo = state.repo_read();
    let project = match repo.get_project_by_id(project_id) {
        Ok(project) => project,
        Err(_) => return Err(Redirect::to(uri!(super::projects::show_projects))),
    };

    let memberships = repo.get_members_by_project(project_id).unwrap_or_default();
    let users = repo.get_users_all().unwrap_or_default();
    drop(repo);

    let owner_count = memberships.iter().filter(|member| member.role == 1).count();
    let can_manage_members = can_manage_members(state, &user, project_id);

    let user_lookup: HashMap<i32, &User> = users.iter().map(|member| (member.id, member)).collect();

    let decorated_members: Vec<Value> = memberships
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

            json!({
                "id": membership.user_id,
                "name": name,
                "username": username,
                "email": email,
                "role_id": membership.role,
                "role_label": describe_project_role(membership.role),
                "is_admin": is_admin,
                "can_remove": can_remove_member(
                    can_manage_members,
                    owner_count,
                    membership,
                    user.id,
                ),
            })
        })
        .collect();

    let member_count = decorated_members.len();

    let member_ids: HashSet<i32> = memberships
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

    let role_options = vec![
        json!({ "id": 1, "label": describe_project_role(1) }),
        json!({ "id": 2, "label": describe_project_role(2) }),
        json!({ "id": 3, "label": describe_project_role(3) }),
        json!({ "id": 4, "label": describe_project_role(4) }),
    ];

    if let Some(ctx_obj) = ctx.as_object_mut() {
        ctx_obj.insert("project".to_string(), json!(project));
        ctx_obj.insert("members".to_string(), json!(decorated_members));
        ctx_obj.insert("can_manage_members".to_string(), json!(can_manage_members));
        ctx_obj.insert("available_users".to_string(), json!(available_users));
        ctx_obj.insert("role_options".to_string(), json!(role_options));
        ctx_obj.insert("project_id".to_string(), json!(project_id));
        ctx_obj.insert("project_slug".to_string(), json!(project_slug));
        ctx_obj.insert("current_user_id".to_string(), json!(user.id));
        ctx_obj.insert("owner_count".to_string(), json!(owner_count));
        ctx_obj.insert("member_count".to_string(), json!(member_count));
        ctx_obj.insert(
            "has_available_users".to_string(),
            json!(has_available_users),
        );
        ctx_obj.insert("selected_project_id".to_string(), json!(project_id));
        ctx_obj.insert("selected_project_slug".to_string(), json!(project_slug));
        ctx_obj.insert(
            "page_title".to_string(),
            json!(format!("{} - Members", project.name)),
        );
    }

    Ok(Template::render("members", ctx))
}

#[post("/<project_id>/members", data = "<form>")]
async fn add_project_member(
    project_access: HtmlProjectAccess,
    project_id: String,
    form: Form<ProjectMemberForm>,
    state: &State<AppState>,
) -> Redirect {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();

    if !can_manage_members(state, &user, project_id) {
        return Redirect::to(format!("/p/{project_slug}/members"));
    }

    let payload = form.into_inner();
    let new_member = NewProjectMember {
        project_id,
        user_id: payload.id,
        role: payload.role,
    };

    if let Err(error) = state.repo_write().add_project_member(&new_member) {
        eprintln!("Error adding project member: {:?}", error);
    }

    Redirect::to(format!("/p/{project_slug}/members"))
}

#[post("/<project_id>/members/<member_id>/remove")]
async fn remove_project_member(
    project_access: HtmlProjectAccess,
    project_id: String,
    member_id: i32,
    state: &State<AppState>,
) -> Redirect {
    let project_slug = project_id;
    let project_id = project_access.project_id();
    let user = project_access.into_user();

    if !can_manage_members(state, &user, project_id) {
        return Redirect::to(format!("/p/{project_slug}/members"));
    }

    let allow_removal = {
        let repo = state.repo_read();
        let members = repo.get_members_by_project(project_id).unwrap_or_default();
        let owner_count = members.iter().filter(|member| member.role == 1).count();

        !members
            .iter()
            .any(|member| member.user_id == member_id && member.role == 1 && owner_count <= 1)
    };

    if !allow_removal {
        return Redirect::to(format!("/p/{project_slug}/members"));
    }

    if let Err(error) = state
        .repo_write()
        .remove_project_member(project_id, member_id)
    {
        eprintln!("Error removing project member: {:?}", error);
    }

    Redirect::to(format!("/p/{project_slug}/members"))
}

pub fn routes() -> Vec<Route> {
    routes![
        show_project_members,
        add_project_member,
        remove_project_member,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Project, ProjectMember, User};
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use crate::routes::html::project::test_helpers::{
        client_with_routes, get_with_session, post_form_with_session, timestamp, TestAppState,
    };
    use crate::status_enums::ProjectStatus;
    use rocket::http::Status as HttpStatus;
    use rocket::local::asynchronous::Client;

    const OWNER_ID: i32 = 1;
    const MEMBER_ID: i32 = 2;
    const CANDIDATE_ID: i32 = 3;
    const ADMIN_ID: i32 = 4;
    const PROJECT_ID: i32 = 1;

    fn sample_project() -> Project {
        Project {
            id: PROJECT_ID,
            name: "Lunar Lander".into(),
            description: Some("Exploration program".into()),
            creation_date: Some(timestamp()),
            update_date: Some(timestamp()),
            status: ProjectStatus::Active,
            owner_id: Some(OWNER_ID),
            slug: "lunar-lander".into(),
            group_id: None,
        }
    }

    fn owner_user() -> User {
        let mut user = DieselRepoMock::make_user(OWNER_ID, "owner", "");
        user.name = "Mission Owner".into();
        user.username = "owner".into();
        user
    }

    fn member_user() -> User {
        let mut user = DieselRepoMock::make_user(MEMBER_ID, "member", "");
        user.name = "Payload Engineer".into();
        user.username = "member".into();
        user
    }

    fn candidate_user() -> User {
        let mut user = DieselRepoMock::make_user(CANDIDATE_ID, "newhire", "");
        user.name = "Flight Specialist".into();
        user.username = "newhire".into();
        user
    }

    fn admin_user() -> User {
        let mut user = DieselRepoMock::make_user(ADMIN_ID, "admin", "");
        user.name = "Site Administrator".into();
        user.username = "admin".into();
        user.is_admin = true;
        user
    }

    fn base_repo() -> DieselRepoMock {
        let mut repo = DieselRepoMock::default();
        repo.users.insert(OWNER_ID, owner_user());
        repo.users.insert(MEMBER_ID, member_user());
        repo.users.insert(CANDIDATE_ID, candidate_user());
        repo.projects.insert(PROJECT_ID, sample_project());
        repo.project_members.push(ProjectMember {
            project_id: PROJECT_ID,
            user_id: OWNER_ID,
            role: 1,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo.project_members.push(ProjectMember {
            project_id: PROJECT_ID,
            user_id: MEMBER_ID,
            role: 2,
            created_at: timestamp(),
            updated_at: timestamp(),
        });
        repo
    }

    fn repo_with_single_owner() -> DieselRepoMock {
        let mut repo = base_repo();
        repo.project_members
            .retain(|member| member.user_id != MEMBER_ID);
        repo
    }

    /// Same as base_repo but with an admin user (id 4) who is not a project member.
    fn base_repo_with_admin() -> DieselRepoMock {
        let mut repo = base_repo();
        repo.users.insert(ADMIN_ID, admin_user());
        repo
    }

    async fn test_client(repo: DieselRepoMock) -> Client {
        client_with_routes(
            repo,
            routes![
                show_project_members,
                add_project_member,
                remove_project_member
            ],
        )
        .await
    }

    #[rocket::async_test]
    async fn show_project_members_displays_roster_for_owner() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/lunar-lander/members", OWNER_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Project Members"));
        assert!(body.contains("Mission Owner"));
        assert!(body.contains("Payload Engineer"));
        assert!(body.contains("Add a Member"));
    }

    #[rocket::async_test]
    async fn show_project_members_hides_management_for_non_owner() {
        let client = test_client(base_repo()).await;
        let response = get_with_session(&client, "/p/lunar-lander/members", MEMBER_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Project Members"));
        assert!(body.contains("Only project owners and administrators can add or remove members"));
        assert!(!body.contains("Add a Member"));
    }

    #[rocket::async_test]
    async fn show_project_members_displays_management_for_admin() {
        let client = test_client(base_repo_with_admin()).await;
        let response = get_with_session(&client, "/p/lunar-lander/members", ADMIN_ID).await;

        assert_eq!(response.status(), HttpStatus::Ok);
        let body = response.into_string().await.expect("body");
        assert!(body.contains("Project Members"));
        assert!(body.contains("Add a Member"));
    }

    #[rocket::async_test]
    async fn add_project_member_as_admin_persists_membership() {
        let client = test_client(base_repo_with_admin()).await;
        let response =
            post_form_with_session(&client, "/p/lunar-lander/members", "id=3&role=2", ADMIN_ID)
                .await;

        assert_eq!(response.status(), HttpStatus::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/lunar-lander/members")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let members = repo
            .get_members_by_project(PROJECT_ID)
            .expect("project members");
        assert!(members.iter().any(|member| member.user_id == CANDIDATE_ID));
    }

    #[rocket::async_test]
    async fn add_project_member_as_owner_persists_membership() {
        let client = test_client(base_repo()).await;
        let response =
            post_form_with_session(&client, "/p/lunar-lander/members", "id=3&role=2", OWNER_ID)
                .await;

        assert_eq!(response.status(), HttpStatus::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/lunar-lander/members")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let members = repo
            .get_members_by_project(PROJECT_ID)
            .expect("project members");
        assert!(members.iter().any(|member| member.user_id == CANDIDATE_ID));
    }

    #[rocket::async_test]
    async fn add_project_member_requires_owner_role() {
        let client = test_client(base_repo()).await;
        let response =
            post_form_with_session(&client, "/p/lunar-lander/members", "id=3&role=2", MEMBER_ID)
                .await;

        assert_eq!(response.status(), HttpStatus::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/lunar-lander/members")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let members = repo
            .get_members_by_project(PROJECT_ID)
            .expect("project members");
        assert_eq!(
            members
                .iter()
                .filter(|member| member.project_id == PROJECT_ID)
                .count(),
            2
        );
        assert!(members.iter().all(|member| member.user_id != CANDIDATE_ID));
    }

    #[rocket::async_test]
    async fn remove_project_member_removes_non_owner() {
        let client = test_client(base_repo()).await;
        let response =
            post_form_with_session(&client, "/p/lunar-lander/members/2/remove", "", OWNER_ID).await;

        assert_eq!(response.status(), HttpStatus::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/lunar-lander/members")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let members = repo
            .get_members_by_project(PROJECT_ID)
            .expect("project members");
        assert!(members.iter().all(|member| member.user_id != MEMBER_ID));
    }

    #[rocket::async_test]
    async fn remove_project_member_prevents_last_owner_removal() {
        let client = test_client(repo_with_single_owner()).await;
        let response =
            post_form_with_session(&client, "/p/lunar-lander/members/1/remove", "", OWNER_ID).await;

        assert_eq!(response.status(), HttpStatus::SeeOther);
        assert_eq!(
            response.headers().get_one("Location"),
            Some("/p/lunar-lander/members")
        );

        let state = client.rocket().state::<TestAppState>().expect("state");
        let repo = state.repo.read().expect("repo lock");
        let members = repo
            .get_members_by_project(PROJECT_ID)
            .expect("project members");
        assert!(members.iter().any(|member| member.user_id == OWNER_ID));
        assert_eq!(members.len(), 1);
    }
}
