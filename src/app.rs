#[cfg(test)]
use crate::repository::diesel_repo_mock::DieselRepoMock;
use crate::repository::CacheRepository;
#[cfg(not(test))]
use crate::repository::DieselRepo;
use rocket::{Build, Rocket};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

#[cfg(not(test))]
pub type DieselCachedRepo = CacheRepository<crate::repository::diesel_repo::DieselRepo>;

#[cfg(test)]
pub type DieselCachedRepo = CacheRepository<crate::repository::diesel_repo_mock::DieselRepoMock>;

#[derive(Clone)]
pub struct AppState<R = DieselCachedRepo> {
    pub repo: Arc<RwLock<R>>,
}

impl AppState<DieselCachedRepo> {
    pub fn repo_read(&self) -> RwLockReadGuard<'_, DieselCachedRepo> {
        self.repo.read().expect("repo lock poisoned")
    }

    pub fn repo_write(&self) -> RwLockWriteGuard<'_, DieselCachedRepo> {
        self.repo.write().expect("repo lock poisoned")
    }
}

#[rocket_sync_db_pools::database("my_db")]
pub struct MyDbConn(rocket_sync_db_pools::diesel::PgConnection);

pub fn build() -> Rocket<Build> {
    let cached = DieselCachedRepo::new(default_inner_repo(), 5 * 60);
    let repo = Arc::new(RwLock::new(cached));

    {
        let repo_guard = repo.write().expect("repo lock poisoned");
        repo_guard.warm_cache();
        repo_guard.cache().start_cache_maintenance();
    }

    rocket::build()
        .manage(AppState { repo })
        .mount("/", crate::routes::html::routes())
        .mount("/p", crate::routes::html::applicability::routes())
        .mount("/p", crate::routes::html::categories::routes())
        .mount("/p", crate::routes::html::requirements::routes())
        .mount("/p", crate::routes::html::tests::routes())
        .mount("/p", crate::routes::html::reports::routes())
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
        .attach(crate::html::cors::CorsFairing)
        .attach(rocket_dyn_templates::Template::fairing())
        .attach(crate::app::MyDbConn::fairing())
}

#[cfg(not(test))]
fn default_inner_repo() -> DieselRepo {
    DieselRepo::new()
}

#[cfg(test)]
fn default_inner_repo() -> DieselRepoMock {
    DieselRepoMock::default()
}
