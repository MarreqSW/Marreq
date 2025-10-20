use crate::repository::errors::RepoError;
use crate::services::log_service::LogServiceError;
use rocket::response::Redirect;
use rocket::serde::json::json;
use rocket::Request;
use rocket_dyn_templates::Template;

#[catch(401)]
pub fn unauthorized(_req: &Request<'_>) -> Template {
    let context = json!({
        "title": "Login",
        "error": "Please log in to continue."
    });

    Template::render("login", context)
}

#[catch(403)]
pub fn forbidden(_req: &Request<'_>) -> Template {
    let context = json!({
        "title": "Access Denied"
    });

    Template::render("access_denied", context)
}

impl From<RepoError> for Redirect {
    fn from(err: RepoError) -> Self {
        println!("Redirecting to error page due to: {}", err);
        Redirect::to("error")
    }
}

impl From<LogServiceError> for Redirect {
    fn from(err: LogServiceError) -> Self {
        println!("Redirecting to error page due to: {}", err);
        Redirect::to("error")
    }
}
