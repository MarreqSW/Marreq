use crate::app::AppState;
use crate::auth::session::SESSION_COOKIE;
use crate::repository::{diesel_repo_mock::DieselRepoMock, CacheRepository};
use chrono::{NaiveDate, NaiveDateTime};
use rocket::http::{ContentType, Cookie};
use rocket::local::asynchronous::{Client, LocalResponse};
use rocket::Route;
use rocket_dyn_templates::Template;
use std::sync::{Arc, RwLock};

pub type TestAppState = AppState<CacheRepository<DieselRepoMock>>;

pub fn timestamp() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

pub fn managed_state(repo: DieselRepoMock) -> TestAppState {
    AppState {
        repo: Arc::new(RwLock::new(CacheRepository::new(repo, 0))),
    }
}

pub async fn client_with_routes(repo: DieselRepoMock, routes: Vec<Route>) -> Client {
    let rocket = rocket::build()
        .manage(managed_state(repo))
        .attach(Template::fairing())
        .mount("/p", routes);

    Client::tracked(rocket).await.expect("rocket instance")
}

pub fn session_cookie(id: i32) -> Cookie<'static> {
    let mut cookie = Cookie::new(SESSION_COOKIE, id.to_string());
    cookie.set_path("/");
    cookie
}

pub async fn get_with_session<'c>(
    client: &'c Client,
    path: &'c str,
    id: i32,
) -> LocalResponse<'c> {
    client
        .get(path)
        .private_cookie(session_cookie(id))
        .dispatch()
        .await
}

pub async fn post_form_with_session<'c>(
    client: &'c Client,
    path: &'c str,
    body: &'c str,
    id: i32,
) -> LocalResponse<'c> {
    client
        .post(path)
        .header(ContentType::Form)
        .private_cookie(session_cookie(id))
        .body(body)
        .dispatch()
        .await
}

pub async fn delete_with_session<'c>(
    client: &'c Client,
    path: &'c str,
    id: i32,
) -> LocalResponse<'c> {
    client
        .delete(path)
        .private_cookie(session_cookie(id))
        .dispatch()
        .await
}
