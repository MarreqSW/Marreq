use super::helpers::decorate_projects_for_listing;
use super::prelude::*;
use crate::services::{ProjectService, StatusService};
use rocket::http::Cookie;

#[get("/")]
pub fn index(
    session_user: SessionUser,
    cookies: &CookieJar<'_>,
    state: &State<AppState>,
) -> Result<Template, Redirect> {
    let user = session_user.into_inner();
    let projects = ProjectService::new(state.inner())
        .get_by_user_id(user.user_id)
        .unwrap_or_default();

    let mut selected_project_id = cookies
        .get("selected_project_id")
        .and_then(|cookie| cookie.value().parse::<i32>().ok());

    // Auto-select first project if none selected and user has projects
    if selected_project_id.is_none() && !projects.is_empty() {
        selected_project_id = Some(projects[0].project_id);
        cookies.add(Cookie::new(
            "selected_project_id",
            projects[0].project_id.to_string(),
        ));
    }
    let decorated_projects = decorate_projects_for_listing(state, &user, &projects);

    let ctx = json!({
        "projects": decorated_projects,
        "projects_count": decorated_projects.len(),
        "user": user,
        "selected_project_id": selected_project_id,
        "hide_nav": true
    });

    Ok(Template::render("index", ctx))
}

#[get("/status")]
pub fn show_status(state: &State<AppState>) -> content::RawHtml<String> {
    let mut out_str = print_header();
    let status_service = StatusService::new(state.inner());

    let all_status = match status_service.list_requirement_statuses() {
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
            out_str, st.req_st_id, st.req_st_title, st.req_st_description
        );
    }

    out_str = format!("{} {}", out_str, print_footer());
    content::RawHtml(out_str)
}

pub fn routes() -> Vec<Route> {
    routes![index, show_status]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::session::SESSION_COOKIE;
    use crate::models::{Project, ProjectMember, Requirement, RequirementStatus, Test, User};
    use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
    use chrono::{NaiveDate, NaiveDateTime};
    use rocket::http::{Cookie, Status};
    use rocket::local::asynchronous::Client;
    use rocket_dyn_templates::Template;
    use std::sync::{Arc, RwLock};

    type TestAppState = AppState<CacheRepository<DieselRepoMock>>;

    fn timestamp() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }

    fn state_with_repo(repo: DieselRepoMock) -> TestAppState {
        AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        }
    }

    async fn test_client(repo: DieselRepoMock) -> Client {
        let rocket = rocket::build()
            .manage(state_with_repo(repo))
            .attach(Template::fairing())
            .mount("/", routes![super::index, super::show_status]);

        Client::tracked(rocket).await.expect("rocket instance")
    }

    fn session_cookie(user_id: i32) -> Cookie<'static> {
        let mut cookie = Cookie::new(SESSION_COOKIE, user_id.to_string());
        cookie.set_path("/");
        cookie.set_http_only(true);
        cookie
    }

    fn dashboard_repo() -> (DieselRepoMock, User) {
        let mut repo = DieselRepoMock::default();

        let user = DieselRepoMock::make_user(1, "jane", "");
        repo.users.insert(user.user_id, user.clone());

        let project = Project {
            project_id: 7,
            project_name: "Project Phoenix".into(),
            project_description: Some("Mission critical".into()),
            project_creation_date: None,
            project_update_date: None,
            project_status: Some("Active".into()),
            project_owner_id: Some(user.user_id),
        };
        repo.projects.insert(project.project_id, project);

        let created = timestamp();
        repo.project_members.push(ProjectMember {
            project_id: 7,
            user_id: user.user_id,
            role: 2,
            created_at: created,
            updated_at: created,
        });

        fn requirement(id: i32, project_id: i32, created: NaiveDateTime) -> Requirement {
            Requirement {
                req_id: id,
                req_title: format!("Requirement {id}"),
                req_description: "Ensure feature works".into(),
                req_verification: 1,
                req_current_status: 1,
                req_author: 1,
                req_reviewer: 1,
                req_link: format!("https://example.com/requirements/{id}"),
                req_reference: format!("REQ-{id}"),
                req_category: 1,
                req_parent: 0,
                req_creation_date: created,
                req_update_date: created,
                req_deadline_date: created,
                req_applicability: 1,
                req_justification: None,
                project_id,
            }
        }

        fn test_case(id: i32, project_id: i32) -> Test {
            Test {
                test_id: id,
                test_name: format!("Test {id}"),
                test_description: "Covers core scenario".into(),
                test_source: "manual".into(),
                test_status: 1,
                test_reference: format!("TST-{id}"),
                test_parent: 0,
                project_id,
            }
        }

        repo.requirements.insert(1, requirement(1, 7, created));
        repo.requirements.insert(2, requirement(2, 7, created));
        repo.tests.insert(1, test_case(1, 7));

        repo.requirement_statuses.insert(
            10,
            RequirementStatus {
                req_st_id: 10,
                req_st_title: "Approved".into(),
                req_st_description: "Ready for release".into(),
                req_st_short_name: "APR".into(),
            },
        );

        (repo, user)
    }

    #[rocket::async_test]
    async fn index_renders_project_overview_with_counts() {
        let (repo, user) = dashboard_repo();
        let client = test_client(repo).await;

        let response = client
            .get("/")
            .private_cookie(session_cookie(user.user_id))
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Project Phoenix"));
        assert!(body.contains("Requirements"));
        assert!(body.contains("href=\"/p/7/requirements\""));
    }

    #[rocket::async_test]
    async fn show_status_lists_requirement_statuses() {
        let (repo, _) = dashboard_repo();
        let client = test_client(repo).await;

        let response = client.get("/status").dispatch().await;

        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("response body");
        assert!(body.contains("Id: 10"));
        assert!(body.contains("Title: Approved"));
        assert!(body.contains("Description: Ready for release"));
    }
}
