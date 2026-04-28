// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! Diesel implementations for the deployment-mode-related repositories
//! (`WorkspacesRepository`, `EmailTokensRepository`) and a few user lookups
//! introduced for Cloud-mode flows. Kept in a separate file so that the
//! large `diesel_repo.rs` does not need to be touched.

use super::diesel_repo::DieselRepo;
use super::errors::RepoError;
use super::{EmailTokensRepository, WorkspacesRepository};
use crate::models::entities::{EmailToken, User, Workspace};
use crate::models::forms::{NewEmailToken, NewWorkspace};
use crate::schema;
use diesel::prelude::*;

impl WorkspacesRepository for DieselRepo {
    fn insert_workspace(&mut self, new: &NewWorkspace) -> Result<i32, RepoError> {
        use schema::workspaces::dsl;
        let mut conn = self.get_conn()?;
        let inserted: Workspace = diesel::insert_into(dsl::workspaces)
            .values(new)
            .get_result(conn.as_mut())?;
        Ok(inserted.id)
    }

    fn get_workspace_by_id(&self, id: i32) -> Result<Workspace, RepoError> {
        use schema::workspaces::dsl;
        let mut conn = self.get_conn()?;
        dsl::workspaces
            .filter(dsl::id.eq(id))
            .first::<Workspace>(conn.as_mut())
            .map_err(|e| match e {
                diesel::result::Error::NotFound => RepoError::NotFound,
                other => RepoError::from(other),
            })
    }

    fn get_workspace_by_slug(&self, slug: &str) -> Result<Option<Workspace>, RepoError> {
        use schema::workspaces::dsl;
        let mut conn = self.get_conn()?;
        dsl::workspaces
            .filter(dsl::slug.eq(slug))
            .first::<Workspace>(conn.as_mut())
            .optional()
            .map_err(RepoError::from)
    }

    fn get_personal_workspace_for_user(
        &self,
        user_id: i32,
    ) -> Result<Option<Workspace>, RepoError> {
        use schema::workspaces::dsl;
        let mut conn = self.get_conn()?;
        dsl::workspaces
            .filter(dsl::owner_user_id.eq(user_id))
            .filter(dsl::kind.eq("personal"))
            .first::<Workspace>(conn.as_mut())
            .optional()
            .map_err(RepoError::from)
    }
}

impl EmailTokensRepository for DieselRepo {
    fn insert_email_token(&mut self, new: &NewEmailToken) -> Result<i32, RepoError> {
        use schema::email_tokens::dsl;
        let mut conn = self.get_conn()?;
        let inserted: EmailToken = diesel::insert_into(dsl::email_tokens)
            .values(new)
            .get_result(conn.as_mut())?;
        Ok(inserted.id)
    }

    fn find_email_token_by_hash(&self, token_hash: &str) -> Result<Option<EmailToken>, RepoError> {
        use schema::email_tokens::dsl;
        let mut conn = self.get_conn()?;
        dsl::email_tokens
            .filter(dsl::token_hash.eq(token_hash))
            .first::<EmailToken>(conn.as_mut())
            .optional()
            .map_err(RepoError::from)
    }

    fn mark_email_token_used(&mut self, id: i32) -> Result<(), RepoError> {
        use schema::email_tokens::dsl;
        let mut conn = self.get_conn()?;
        let now = chrono::Utc::now().naive_utc();
        diesel::update(dsl::email_tokens.filter(dsl::id.eq(id)))
            .set(dsl::used_at.eq(now))
            .execute(conn.as_mut())?;
        Ok(())
    }
}

/// Extra `UserRepository` methods on the real Diesel-backed repo that were
/// added for Cloud-mode flows. The original implementation in
/// `diesel_repo.rs` already provides the rest of the trait; here we tack on a
/// supplementary `impl` block with the new methods.
impl DieselRepo {
    pub(crate) fn db_get_user_by_email(&self, email: &str) -> Result<Option<User>, RepoError> {
        use schema::users::dsl;
        let mut conn = self.get_conn()?;
        // Case-insensitive lookup against the lower(email) functional index.
        let normalized = email.trim().to_lowercase();
        dsl::users
            .filter(dsl::email.eq(&normalized))
            .first::<User>(conn.as_mut())
            .optional()
            .map_err(RepoError::from)
    }

    pub(crate) fn db_set_user_email_verified(
        &self,
        user_id: i32,
        verified: bool,
    ) -> Result<(), RepoError> {
        use schema::users::dsl;
        let mut conn = self.get_conn()?;
        let affected = diesel::update(dsl::users.filter(dsl::id.eq(user_id)))
            .set(dsl::email_verified.eq(verified))
            .execute(conn.as_mut())?;
        if affected == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }
}

impl super::SessionRepository for DieselRepo {
    fn create_session(
        &mut self,
        new: &crate::models::entities::NewSession,
    ) -> Result<(), RepoError> {
        use schema::sessions::dsl;
        let mut conn = self.get_conn()?;
        diesel::insert_into(dsl::sessions)
            .values(new)
            .execute(conn.as_mut())?;
        Ok(())
    }

    fn find_active_session(
        &self,
        token_hash: &str,
        now: chrono::NaiveDateTime,
    ) -> Result<Option<crate::models::entities::Session>, RepoError> {
        use schema::sessions::dsl;
        let mut conn = self.get_conn()?;
        dsl::sessions
            .filter(dsl::token_hash.eq(token_hash))
            .filter(dsl::expires_at.gt(now))
            .first::<crate::models::entities::Session>(conn.as_mut())
            .optional()
            .map_err(RepoError::from)
    }

    fn touch_session(
        &mut self,
        token_hash: &str,
        now: chrono::NaiveDateTime,
    ) -> Result<(), RepoError> {
        use schema::sessions::dsl;
        let mut conn = self.get_conn()?;
        diesel::update(dsl::sessions.filter(dsl::token_hash.eq(token_hash)))
            .set(dsl::last_seen_at.eq(now))
            .execute(conn.as_mut())?;
        Ok(())
    }

    fn delete_session(&mut self, token_hash: &str) -> Result<(), RepoError> {
        use schema::sessions::dsl;
        let mut conn = self.get_conn()?;
        diesel::delete(dsl::sessions.filter(dsl::token_hash.eq(token_hash)))
            .execute(conn.as_mut())?;
        Ok(())
    }

    fn delete_user_sessions(&mut self, user_id: i32) -> Result<(), RepoError> {
        use schema::sessions::dsl;
        let mut conn = self.get_conn()?;
        diesel::delete(dsl::sessions.filter(dsl::user_id.eq(user_id))).execute(conn.as_mut())?;
        Ok(())
    }

    fn purge_expired_sessions(&mut self, now: chrono::NaiveDateTime) -> Result<usize, RepoError> {
        use schema::sessions::dsl;
        let mut conn = self.get_conn()?;
        let n =
            diesel::delete(dsl::sessions.filter(dsl::expires_at.le(now))).execute(conn.as_mut())?;
        Ok(n)
    }
}
