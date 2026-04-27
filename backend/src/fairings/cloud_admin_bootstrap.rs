// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Cloud-only Rocket fairing that ensures the configured site administrator
//! exists and is flagged as `is_admin = true` and `email_verified = true`.

use rocket::fairing::{self, Fairing, Info, Kind};
use rocket::{Build, Rocket};

use crate::app::AppState;
use crate::auth::password::hash_password;
use crate::models::NewUser;
use crate::repository::UserRepository;

pub struct CloudAdminBootstrapFairing;

#[rocket::async_trait]
impl Fairing for CloudAdminBootstrapFairing {
    fn info(&self) -> Info {
        Info {
            name: "Cloud site-admin bootstrap",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        let Some(email) = std::env::var("MARREQ_SITE_ADMIN_EMAIL").ok() else {
            rocket::warn!(
                "Cloud mode: MARREQ_SITE_ADMIN_EMAIL is not set; no site administrator will be bootstrapped."
            );
            return Ok(rocket);
        };
        let email_norm = email.trim().to_lowercase();
        if email_norm.is_empty() {
            return Ok(rocket);
        }

        let state = match rocket.state::<AppState>() {
            Some(s) => s.clone(),
            None => {
                rocket::error!("Cloud admin bootstrap: AppState missing; skipping.");
                return Ok(rocket);
            }
        };

        let bootstrap_password = std::env::var("MARREQ_SITE_ADMIN_BOOTSTRAP_PASSWORD").ok();

        let mut repo = state.repo_write();
        match repo.get_user_by_email(&email_norm) {
            Ok(Some(mut existing)) => {
                let mut changed = false;
                if !existing.is_admin {
                    existing.is_admin = true;
                    changed = true;
                }
                if !existing.email_verified {
                    let _ = repo.set_user_email_verified(existing.id, true);
                }
                if changed {
                    let payload = NewUser {
                        id: Some(existing.id),
                        username: existing.username.clone(),
                        name: existing.name.clone(),
                        email: existing.email.clone(),
                        password_hash: existing.password_hash.clone(),
                        is_admin: true,
                        email_verified: Some(true),
                    };
                    if let Err(e) = repo.update_user(&payload) {
                        rocket::error!(
                            "Cloud admin bootstrap: failed to promote {email_norm}: {e}"
                        );
                    } else {
                        rocket::info!("Cloud admin bootstrap: promoted existing user {email_norm} to site admin.");
                    }
                }
            }
            Ok(None) => {
                let Some(pwd) = bootstrap_password.as_deref() else {
                    rocket::warn!(
                        "Cloud admin bootstrap: user {email_norm} does not exist and MARREQ_SITE_ADMIN_BOOTSTRAP_PASSWORD is not set; cannot create site admin."
                    );
                    return Ok(rocket);
                };
                let hash = match hash_password(pwd) {
                    Ok(h) => h,
                    Err(e) => {
                        rocket::error!("Cloud admin bootstrap: password hashing failed: {e}");
                        return Ok(rocket);
                    }
                };
                let username = email_norm.split('@').next().unwrap_or("admin").to_string();
                let new_user = NewUser {
                    id: None,
                    username,
                    name: "Site Administrator".into(),
                    email: email_norm.clone(),
                    password_hash: hash,
                    is_admin: true,
                    email_verified: Some(true),
                };
                match repo.insert_user(&new_user) {
                    Ok(id) => rocket::info!(
                        "Cloud admin bootstrap: created site admin {email_norm} (id={id})."
                    ),
                    Err(e) => rocket::error!(
                        "Cloud admin bootstrap: failed to create site admin {email_norm}: {e}"
                    ),
                }
            }
            Err(e) => {
                rocket::error!("Cloud admin bootstrap: lookup failed: {e}");
            }
        }
        Ok(rocket)
    }
}
