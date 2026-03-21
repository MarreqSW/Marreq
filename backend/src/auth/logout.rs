// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::auth::csrf::clear_csrf_cookie;
use crate::auth::{clear_session_cookie, read_session_user_id};
use crate::models::NewLog;
use crate::repository::LogRepository;
use rocket::http::CookieJar;

/// Clear session cookies and log the logout event.
/// Note: login calls Cookie::new(..). By default, Rocket's Cookie::new creates
/// a cookie without any explicit path, and Rocket's add_private will add it to
/// the response as-is. The default path is handled by the browser: if no Path
/// is given, RFC 6265 says the default is the request's path up to the rightmost "/".
pub fn logout_user<R: LogRepository>(cookies: &CookieJar<'_>, repo: &mut R) {
    // Get user info before clearing cookies
    let user_id = read_session_user_id(cookies);

    // Remove the session cookie and the CSRF token cookie together so that
    // outstanding CSRF tokens cannot be replayed after logout.
    clear_session_cookie(cookies);
    clear_csrf_cookie(cookies);

    // Remove legacy cookies from previous versions if they exist
    for legacy in &["id", "username", "name"] {
        let mut cookie = rocket::http::Cookie::new(*legacy, "");
        cookie.set_path("/");
        cookies.remove_private(cookie);
    }

    // Log logout if we have a user_id - don't fail if logging fails
    if let Some(uid) = user_id {
        let log = NewLog {
            user_id: uid,
            action_type: "LOGOUT".to_string(),
            entity_type: "User".to_string(),
            project_id: None,
            entity_id: Some(uid),
            old_values: None,
            new_values: None,
            description: Some("User logged out".to_string()),
            ip_address: None,
            user_agent: None,
        };
        let _ = repo.insert_log(&log);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use rocket::local::blocking::Client;
    use rocket::{get, routes};

    #[get("/")]
    fn set_basic(cookies: &CookieJar<'_>) {
        cookies.add_private(rocket::http::Cookie::new("id", "not-an-int"));
        cookies.add_private(rocket::http::Cookie::new("username", "alice"));
        cookies.add_private(rocket::http::Cookie::new("name", "Alice"));
    }

    #[test]
    fn removes_session_cookies() {
        let rocket = rocket::build().mount("/", routes![set_basic]);
        let client = Client::tracked(rocket).expect("valid rocket");
        client.get("/").dispatch();
        let jar = client.cookies();
        assert!(jar.get_private("username").is_some());

        let mut repo = DieselRepoMock::default();
        logout_user(&jar, &mut repo);

        assert!(jar.get_pending("id").is_none());
        assert!(jar.get_pending("username").is_none());
        assert!(jar.get_pending("name").is_none());
    }

    #[get("/")]
    fn set_with_other(cookies: &CookieJar<'_>) {
        cookies.add(rocket::http::Cookie::new("other", "ok"));
        cookies.add_private(rocket::http::Cookie::new("id", "bad"));
        cookies.add_private(rocket::http::Cookie::new("username", "bob"));
        cookies.add_private(rocket::http::Cookie::new("name", "Bob"));
    }

    #[test]
    fn leaves_other_cookies_intact() {
        let rocket = rocket::build().mount("/", routes![set_with_other]);
        let client = Client::tracked(rocket).expect("valid rocket");
        client.get("/").dispatch();
        let jar = client.cookies();

        let mut repo = DieselRepoMock::default();
        logout_user(&jar, &mut repo);

        assert!(jar.get_pending("id").is_none());
        assert!(jar.get_pending("username").is_none());
        assert!(jar.get_pending("name").is_none());
        assert!(jar.get_pending("other").is_some());
    }
}
