// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

#[cfg(any(test, feature = "test-helpers"))]
use crate::repository::diesel_repo_mock::DieselRepoMock;
use crate::repository::errors::RepoError;
use crate::repository::CacheRepository;
#[cfg(not(any(test, feature = "test-helpers")))]
use crate::repository::DieselRepo;
use rocket::{Build, Rocket};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

#[cfg(not(any(test, feature = "test-helpers")))]
pub type DieselCachedRepo = CacheRepository<crate::repository::diesel_repo::DieselRepo>;

#[cfg(any(test, feature = "test-helpers"))]
pub type DieselCachedRepo = CacheRepository<crate::repository::diesel_repo_mock::DieselRepoMock>;

pub struct AppState<R = DieselCachedRepo> {
    pub repo: Arc<RwLock<R>>,
}

impl<R> Clone for AppState<R> {
    fn clone(&self) -> Self {
        Self {
            repo: Arc::clone(&self.repo),
        }
    }
}

impl AppState<DieselCachedRepo> {
    pub fn repo_read(&self) -> RwLockReadGuard<'_, DieselCachedRepo> {
        self.repo.read().expect("repo lock poisoned")
    }

    pub fn repo_write(&self) -> RwLockWriteGuard<'_, DieselCachedRepo> {
        self.repo.write().expect("repo lock poisoned")
    }

    /// Non-panicking read access; use in request path (e.g. guards) to return 500 instead of panicking on poisoned lock.
    pub fn try_repo_read(&self) -> Result<RwLockReadGuard<'_, DieselCachedRepo>, RepoError> {
        self.repo
            .read()
            .map_err(|_| RepoError::Pool("repo lock poisoned".into()))
    }

    /// Non-panicking write access; use in request path when lock failure should yield 500 instead of panic.
    pub fn try_repo_write(&self) -> Result<RwLockWriteGuard<'_, DieselCachedRepo>, RepoError> {
        self.repo
            .write()
            .map_err(|_| RepoError::Pool("repo lock poisoned".into()))
    }
}

#[rocket_sync_db_pools::database("my_db")]
pub struct MyDbConn(rocket_sync_db_pools::diesel::PgConnection);

pub fn build_with(
    mode: &'static dyn crate::deployment::DeploymentMode,
    extra_routes: Vec<(crate::api::RoutePolicy, rocket::Route)>,
    extra_fairings: Vec<std::sync::Arc<dyn rocket::fairing::Fairing>>,
) -> Rocket<Build> {
    let auth_config = crate::auth::AuthConfig::from_env(mode.name()).unwrap_or_else(|error| {
        eprintln!("Invalid authentication configuration: {error}");
        std::process::exit(2);
    });
    build_with_auth(mode, auth_config, extra_routes, extra_fairings)
}

/// Policy-classified routes grouped by the mount base `build_with_auth` uses.
#[derive(Debug)]
pub struct DeclaredMounts {
    /// Mounted at `/api`.
    pub api: Vec<(crate::api::RoutePolicy, rocket::Route)>,
    /// Mounted at `/` (root handlers + OAuth/well-known).
    pub root: Vec<(crate::api::RoutePolicy, rocket::Route)>,
}

/// Assemble the exact route providers that `build_with_auth` mounts.
pub fn declared_mounts(
    extra_routes: Vec<(crate::api::RoutePolicy, rocket::Route)>,
) -> DeclaredMounts {
    let mut api = crate::api::routes_with_policies();
    api.extend(extra_routes);
    let mut root = crate::api::root_routes_with_policies();
    root.extend(crate::api::oauth::routes_with_policies());
    DeclaredMounts { api, root }
}

/// Flat declared route-policy inventory for the routes `build_with_auth` mounts.
pub fn declared_route_inventory(
    extra_routes: Vec<(crate::api::RoutePolicy, rocket::Route)>,
) -> Vec<(crate::api::RoutePolicy, rocket::Route)> {
    let mounts = declared_mounts(extra_routes);
    let mut declared = mounts.api;
    declared.extend(mounts.root);
    declared
}

pub fn build_with_auth(
    mode: &'static dyn crate::deployment::DeploymentMode,
    auth_config: crate::auth::AuthConfig,
    extra_routes: Vec<(crate::api::RoutePolicy, rocket::Route)>,
    extra_fairings: Vec<std::sync::Arc<dyn rocket::fairing::Fairing>>,
) -> Rocket<Build> {
    // Register the mode into the OnceLock so `deployment::current()` works
    // without per-call lookups.
    crate::deployment::set_current(mode);
    eprintln!("[marreq] deployment mode: {}", mode.name());

    #[cfg(not(any(test, feature = "test-helpers")))]
    let inner = {
        crate::repository::diesel_repo::init_connection_pool().unwrap_or_else(|e| {
            eprintln!("Database setup failed: {}", e);
            std::process::exit(1);
        });
        default_inner_repo().unwrap_or_else(|e| {
            eprintln!("Database setup failed: {}", e);
            std::process::exit(1);
        })
    };
    #[cfg(any(test, feature = "test-helpers"))]
    let inner = default_inner_repo();
    let cached = DieselCachedRepo::new(inner, 5 * 60);
    let repo = Arc::new(RwLock::new(cached));

    {
        let repo_guard = repo.write().expect("repo lock poisoned");
        repo_guard.warm_cache();
        repo_guard.cache().start_cache_maintenance();
    }

    let mounts = declared_mounts(extra_routes);
    let api_routes = mounts
        .api
        .into_iter()
        .map(|(_, route)| route)
        .collect::<Vec<_>>();
    let root_routes = mounts
        .root
        .into_iter()
        .map(|(_, route)| route)
        .collect::<Vec<_>>();

    let mut rocket = rocket::build()
        .manage(AppState { repo })
        .manage(auth_config)
        .manage(mode)
        .manage(crate::auth::rate_limiter::LoginRateLimiter::new())
        .manage(crate::api::oauth::OAuthRegistrationRateLimiter::new())
        .mount("/", root_routes)
        .mount("/api", api_routes)
        .register(
            "/",
            catchers![
                crate::routes::catchers::unauthorized,
                crate::routes::catchers::forbidden,
                crate::routes::catchers::not_found,
            ],
        )
        .attach(crate::fairings::SecurityHeadersFairing)
        .attach(crate::fairings::RequestLogFairing)
        .attach(crate::fairings::OAuthChallengeFairing)
        .attach(crate::fairings::CsrfFairing::new())
        .attach(crate::cors::CorsFairing(crate::cors::CorsPolicy::from_env()))
        .attach(crate::fairings::AntiCacheFairing)
        .attach(crate::fairings::SemanticIndexFairing);

    #[cfg(not(any(test, feature = "test-helpers")))]
    {
        rocket = rocket.attach(crate::app::MyDbConn::fairing());
    }

    for fairing in extra_fairings {
        rocket = rocket.attach(fairing);
    }

    rocket
}

#[cfg(not(any(test, feature = "test-helpers")))]
fn default_inner_repo() -> Result<DieselRepo, Box<dyn std::error::Error>> {
    DieselRepo::new()
}

#[cfg(any(test, feature = "test-helpers"))]
fn default_inner_repo() -> DieselRepoMock {
    DieselRepoMock::default()
}

#[cfg(test)]
mod policy_inventory_tests {
    use super::*;
    use crate::api::RoutePolicy;
    use crate::deployment::install_test_server_mode;
    use rocket::http::Method;
    use std::collections::BTreeSet;

    fn route_key(method: Method, uri: &str) -> String {
        format!("{method} {uri}")
    }

    fn join_mount(base: &str, uri: &str) -> String {
        if base == "/" {
            return uri.to_string();
        }
        let base = base.trim_end_matches('/');
        if uri == "/" {
            return base.to_string();
        }
        format!("{base}{uri}")
    }

    #[test]
    fn every_mounted_route_has_exactly_one_declared_policy() {
        install_test_server_mode();
        let mounts = declared_mounts(vec![]);
        let mut declared_keys = BTreeSet::new();
        for (policy, route) in &mounts.api {
            assert!(route.name.is_some(), "route missing name under {policy:?}");
            let key = route_key(route.method, &join_mount("/api", &route.uri.to_string()));
            assert!(
                declared_keys.insert(key.clone()),
                "duplicate policy declaration for {key}"
            );
        }
        for (policy, route) in &mounts.root {
            assert!(route.name.is_some(), "route missing name under {policy:?}");
            let key = route_key(route.method, &join_mount("/", &route.uri.to_string()));
            assert!(
                declared_keys.insert(key.clone()),
                "duplicate policy declaration for {key}"
            );
        }

        let rocket = build_with(crate::deployment::current(), vec![], vec![]);
        let mut mounted_keys = BTreeSet::new();
        for route in rocket.routes() {
            let key = route_key(route.method, &route.uri.to_string());
            assert!(
                mounted_keys.insert(key.clone()),
                "duplicate mounted route for {key}"
            );
        }

        assert_eq!(
            declared_keys, mounted_keys,
            "declared route-policy inventory must match the Rocket route table"
        );
        assert!(
            declared_route_inventory(vec![]).iter().any(|(p, r)| {
                *p == RoutePolicy::Authenticated && r.name.as_deref() == Some("grants")
            }),
            "oauth grants must be present in the policy inventory"
        );
    }
}
