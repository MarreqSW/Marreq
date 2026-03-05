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

pub fn build() -> Rocket<Build> {
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

    rocket::build()
        .manage(AppState { repo })
        .mount("/", crate::routes::html::routes())
        .mount("/", routes![crate::fairings::csrf_denied])
        .mount("/p", crate::routes::html::project::routes())
        .mount("/user", crate::routes::html::user::routes())
        .mount("/api", crate::api::routes())
        .register(
            "/",
            catchers![
                crate::routes::catchers::unauthorized,
                crate::routes::catchers::forbidden
            ],
        )
        .mount(
            "/static",
            rocket::fs::FileServer::from(rocket::fs::relative!("src/html/static")),
        )
        .attach(crate::fairings::SecurityHeadersFairing)
        .attach(crate::fairings::CsrfFairing::new())
        .attach(crate::html::cors::CorsFairing(
            crate::html::cors::CorsPolicy::from_env(),
        ))
        .attach(crate::fairings::AntiCacheFairing)
        .attach(crate::fairings::SemanticIndexFairing)
        .attach(rocket_dyn_templates::Template::fairing())
        .attach(crate::app::MyDbConn::fairing())
}

#[cfg(not(any(test, feature = "test-helpers")))]
fn default_inner_repo() -> Result<DieselRepo, Box<dyn std::error::Error>> {
    DieselRepo::new()
}

#[cfg(any(test, feature = "test-helpers"))]
fn default_inner_repo() -> DieselRepoMock {
    DieselRepoMock::default()
}
