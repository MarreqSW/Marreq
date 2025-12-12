use crate::repository::errors::RepoError;
use crate::repository::{LookupRepository, RequirementsRepository};
use rocket::http::CookieJar;

pub fn get_selected_project_id(cookies: &CookieJar<'_>) -> Option<i32> {
    cookies
        .get("selected_project_id")
        .and_then(|cookie| cookie.value().parse::<i32>().ok())
}

pub fn generate_requirement_reference<R>(
    repo: &R,
    category_id: i32,
    project_id: i32,
) -> Result<String, RepoError>
where
    R: LookupRepository + RequirementsRepository,
{
    let category = repo.get_category_by_id(category_id)?;

    let existing_count = repo
        .get_requirements_by_project(project_id)?
        .into_iter()
        .filter(|req| req.category_id == category_id)
        .count();

    Ok(format!("REQ-{}-{}", category.tag, existing_count + 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Category;
    use crate::repository::diesel_repo_mock::DieselRepoMock;
    use rocket::http::Cookie;
    use rocket::local::blocking::Client;
    use rocket::{get, routes};

    #[get("/")]
    fn read_cookie_route(cookies: &CookieJar<'_>) -> String {
        get_selected_project_id(cookies)
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".into())
    }

    #[test]
    fn get_selected_project_id_returns_id() {
        let rocket = rocket::build().mount("/", routes![read_cookie_route]);
        let client = Client::untracked(rocket).expect("valid rocket instance");
        let response = client
            .get("/")
            .cookie(Cookie::new("selected_project_id", "42"))
            .dispatch();
        assert_eq!(response.into_string().unwrap(), "42");
    }

    #[test]
    fn get_selected_project_id_missing_cookie() {
        let rocket = rocket::build().mount("/", routes![read_cookie_route]);
        let client = Client::untracked(rocket).expect("valid rocket instance");
        let response = client.get("/").dispatch();
        assert_eq!(response.into_string().unwrap(), "none");
    }

    #[test]
    fn get_selected_project_id_invalid_cookie() {
        let rocket = rocket::build().mount("/", routes![read_cookie_route]);
        let client = Client::untracked(rocket).expect("valid rocket instance");
        let response = client
            .get("/")
            .cookie(Cookie::new("selected_project_id", "abc"))
            .dispatch();
        assert_eq!(response.into_string().unwrap(), "none");
    }

    #[test]
    fn generate_requirement_reference_creates_incremental_reference() {
        let mut repo = DieselRepoMock::default();
        let project_id = 1;
        let category = Category {
            id: 1,
            title: "Test Cat".into(),
            description: "desc".into(),
            tag: "TC".into(),
            project_id,
        };
        repo.categories.insert(category.id, category.clone());

        let reference = generate_requirement_reference(&repo, category.id, project_id)
            .expect("reference generation");
        assert_eq!(reference, format!("REQ-{}-1", category.tag));
    }

    #[test]
    fn generate_requirement_reference_missing_category_returns_error() {
        let repo = DieselRepoMock::default();
        let result = generate_requirement_reference(&repo, -1, -1);
        assert!(result.is_err());
    }
}
