// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::DieselRepo;
use crate::models::{NewUserIdentity, UserIdentity};
use crate::repository::{ExternalIdentityRepository, RepoError};
use diesel::prelude::*;
use diesel::OptionalExtension;

impl ExternalIdentityRepository for DieselRepo {
    fn get_identity(
        &self,
        issuer_value: &str,
        subject_value: &str,
    ) -> Result<Option<UserIdentity>, RepoError> {
        use crate::schema::user_identities::dsl;
        let mut conn = self.get_conn()?;
        dsl::user_identities
            .filter(dsl::issuer.eq(issuer_value))
            .filter(dsl::subject.eq(subject_value))
            .first(conn.as_mut())
            .optional()
            .map_err(Into::into)
    }
    fn get_identities_for_user(&self, owner: i32) -> Result<Vec<UserIdentity>, RepoError> {
        use crate::schema::user_identities::dsl;
        let mut conn = self.get_conn()?;
        dsl::user_identities
            .filter(dsl::user_id.eq(owner))
            .order(dsl::id)
            .load(conn.as_mut())
            .map_err(Into::into)
    }
    fn insert_identity(&mut self, identity: &NewUserIdentity) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        diesel::insert_into(crate::schema::user_identities::table)
            .values(identity)
            .returning(crate::schema::user_identities::id)
            .get_result(conn.as_mut())
            .map_err(super::map_unique_violation)
    }
    fn touch_identity_login(
        &mut self,
        identity_id: i32,
        now: chrono::NaiveDateTime,
    ) -> Result<(), RepoError> {
        use crate::schema::user_identities::dsl;
        let mut conn = self.get_conn()?;
        diesel::update(dsl::user_identities.filter(dsl::id.eq(identity_id)))
            .set(dsl::last_login_at.eq(now))
            .execute(conn.as_mut())
            .map(|_| ())
            .map_err(Into::into)
    }
    fn delete_identity(&mut self, identity_id: i32, owner: i32) -> Result<bool, RepoError> {
        use crate::schema::user_identities::dsl;
        let mut conn = self.get_conn()?;
        diesel::delete(
            dsl::user_identities
                .filter(dsl::id.eq(identity_id))
                .filter(dsl::user_id.eq(owner)),
        )
        .execute(conn.as_mut())
        .map(|n| n == 1)
        .map_err(Into::into)
    }
}
