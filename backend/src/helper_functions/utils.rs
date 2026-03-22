// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use crate::repository::errors::RepoError;
use crate::repository::{LookupRepository, RequirementsRepository};
use rocket::http::CookieJar;
use std::collections::HashSet;

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

    Ok(format!(
        "REQ-{}-{}",
        category.tag.to_uppercase(),
        existing_count + 1
    ))
}

pub fn slugify_project_name(name: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_dash = false;
            continue;
        }

        if !slug.is_empty() && !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        "project".to_string()
    } else {
        slug
    }
}

pub fn generate_unique_project_slug<I, S>(name: &str, existing_slugs: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let base = slugify_project_name(name);
    let existing: HashSet<String> = existing_slugs
        .into_iter()
        .map(|slug| slug.as_ref().to_string())
        .collect();

    if !existing.contains(&base) {
        return base;
    }

    let mut occurrence = 2;
    loop {
        let suffix = format!("-{occurrence}");
        let candidate = format!("{}{}", &base[..base.len().min(255 - suffix.len())], suffix);

        if !existing.contains(&candidate) {
            return candidate;
        }

        occurrence += 1;
    }
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

    #[test]
    fn slugify_project_name_normalizes_to_kebab_case() {
        assert_eq!(slugify_project_name("Flight Control"), "flight-control");
        assert_eq!(slugify_project_name("My   Project!!!"), "my-project");
    }

    #[test]
    fn slugify_project_name_falls_back_for_empty_result() {
        assert_eq!(slugify_project_name("!!!"), "project");
    }

    #[test]
    fn generate_unique_project_slug_adds_collision_suffixes() {
        let slug =
            generate_unique_project_slug("Flight Control", ["flight-control", "flight-control-2"]);
        assert_eq!(slug, "flight-control-3");
    }
}
