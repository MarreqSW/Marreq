use req_man::models::{NewProject, Project, User, ProjectMember};
use req_man::repository::diesel_repo_mock::DieselRepoMock;
use req_man::routes::html::projects;
use req_man::routes::html::project;
use req_man::auth::session::SESSION_COOKIE;
use rocket::http::{Cookie, Status};
use rocket::local::asynchronous::Client;
use rocket::State;
use req_man::app::AppState;
use req_man::repository::CacheRepository;
use std::sync::{Arc, RwLock};

// Helper to create a test client with a populated mock repository
async fn test_client(repo: DieselRepoMock) -> Client {
    let rocket = rocket::build()
        .manage(AppState {
            repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
        })
        .attach(rocket_dyn_templates::Template::fairing())
        .mount(
            "/",
            rocket::routes![
                projects::show_projects,
                projects::new_project,
                projects::post_project,
            ],
        )
        .mount(
            "/p",
            project::routes(),
        )
        .register(
            "/",
            rocket::catchers![
                req_man::routes::catchers::unauthorized,
                req_man::routes::catchers::forbidden
            ],
        );

    Client::tracked(rocket).await.expect("rocket instance")
}

// Helper to create a basic user and session
fn authenticated_repo(user_id: i32) -> DieselRepoMock {
    let user = DieselRepoMock::make_user(user_id, "testuser", "password");
    DieselRepoMock::with_users([user])
}

fn session_cookie(user_id: i32) -> Cookie<'static> {
    let mut cookie = Cookie::new(SESSION_COOKIE, user_id.to_string());
    cookie.set_path("/");
    cookie
}

#[rocket::async_test]
async fn projects_page_requires_authentication() {
    let client = test_client(DieselRepoMock::default()).await;
    let response = client.get("/projects").dispatch().await;
    
    assert_eq!(response.status(), Status::Unauthorized);
    let body = response.into_string().await.unwrap();
    // Check for the error message set in catchers.rs
    assert!(body.contains("Please log in to continue"));
}

#[rocket::async_test]
async fn projects_page_lists_user_projects() {
    let mut repo = authenticated_repo(1);
    // Add a project where user 1 is owner
    let project = Project {
        project_id: 10,
        project_name: "My Project".into(),
        project_description: Some("Description".into()),
        project_creation_date: None,
        project_update_date: None,
        project_status: Some("Active".into()),
        project_owner_id: Some(1),
    };
    repo.projects.insert(10, project);
    
    // Add membership
    repo.project_members.push(ProjectMember {
        project_id: 10,
        user_id: 1,
        role: 1, // Owner
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    });

    let client = test_client(repo).await;
    let response = client
        .get("/projects")
        .private_cookie(session_cookie(1)) // Use private_cookie
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let body = response.into_string().await.unwrap();
    assert!(body.contains("My Project"));
}

#[rocket::async_test]
async fn create_project_success() {
    let mut repo = authenticated_repo(1);
    let mut admin = DieselRepoMock::make_user(1, "admin", "pass");
    admin.is_admin = true;
    repo.users.insert(1, admin);

    let client = test_client(repo).await;

    let response = client
        .post("/new_project")
        .private_cookie(session_cookie(1))
        .header(rocket::http::ContentType::Form)
        .body("project_name=New+Project&project_description=Test+Description&project_status=active&project_owner_id=1")
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::SeeOther);
    assert_eq!(response.headers().get_one("Location"), Some("/projects"));
}

#[rocket::async_test]
async fn access_project_details_as_owner() {
    let mut repo = authenticated_repo(1);
    let project = Project {
        project_id: 30,
        project_name: "Owner Project".into(),
        project_description: None,
        project_creation_date: None,
        project_update_date: None,
        project_status: Some("Active".into()),
        project_owner_id: Some(1),
    };
    repo.projects.insert(30, project);
    
    // Add membership
    repo.project_members.push(ProjectMember {
        project_id: 30,
        user_id: 1,
        role: 1, // Owner
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    });

    let client = test_client(repo).await;
    let response = client
        .get("/p/30")
        .private_cookie(session_cookie(1))
        .dispatch()
        .await;

    assert_eq!(response.status(), Status::Ok);
    let body = response.into_string().await.unwrap();
    assert!(body.contains("Owner Project"));
}

#[rocket::async_test]
async fn access_project_details_forbidden_for_non_member() {
    let mut repo = authenticated_repo(2); // User 2
    let project = Project {
        project_id: 40,
        project_name: "Private Project".into(),
        project_description: None,
        project_creation_date: None,
        project_update_date: None,
        project_status: Some("Active".into()),
        project_owner_id: Some(1), // Owned by User 1
    };
    repo.projects.insert(40, project);
    // User 2 is NOT a member

    let client = test_client(repo).await;
    let response = client
        .get("/p/40")
        .private_cookie(session_cookie(2))
        .dispatch()
        .await;

    // With catchers, this should return 403 Forbidden
    assert_eq!(response.status(), Status::Forbidden);
    let body = response.into_string().await.unwrap();
    assert!(body.contains("Access Denied"));
}
