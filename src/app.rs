use crate::repository::{DieselCachedRepo, DieselRepo};
use rocket::{Build, Rocket};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<RwLock<DieselCachedRepo>>,
}

impl AppState {
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
    let cached = DieselCachedRepo::new(DieselRepo::new(), 5 * 60);
    let repo = Arc::new(RwLock::new(cached));

    {
        let repo_guard = repo.write().expect("repo lock poisoned");
        repo_guard.warm_cache();
        repo_guard.cache().start_cache_maintenance();
    }

    rocket::build()
        .manage(AppState { repo })
        .mount("/", crate::routes::html::routes())
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
