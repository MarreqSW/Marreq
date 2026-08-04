// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

use super::lower;
use super::{map_unique_violation, DieselRepo};
use crate::models::entities::*;
use crate::models::forms::*;
use crate::repository::errors::RepoError;
use crate::repository::{ApiTokensRepository, UserRepository};
use crate::schema;
use diesel::prelude::*;
use diesel::{Connection, JoinOnDsl, OptionalExtension};

impl UserRepository for DieselRepo {
    fn get_users_all(&self) -> Result<Vec<User>, RepoError> {
        use schema::users::dsl;
        let mut conn = self.get_conn()?;
        dsl::users
            .order(dsl::id)
            .load::<User>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_user_by_id(&self, user_id: i32) -> Result<User, RepoError> {
        use schema::users::dsl;
        let mut conn = self.get_conn()?;

        dsl::users
            .filter(dsl::id.eq(user_id))
            .first::<User>(conn.as_mut()) // <-- use inner PgConnection
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_user_by_username(&self, uname: &str) -> Result<Option<User>, RepoError> {
        use crate::schema::users::dsl;
        let mut conn = self.get_conn()?;
        // Case-insensitive lookup so "alice" / "Alice" / "ALICE" all work (uname is already lowercased by auth).
        dsl::users
            .filter(lower(dsl::username).eq(uname))
            .first::<User>(conn.as_mut())
            .optional()
            .map_err(|e| e.into())
    }

    fn update_user_password(&mut self, user_id: i32, new_hash: &str) -> Result<(), RepoError> {
        use crate::schema::users::dsl;
        let mut conn = self.get_conn()?;

        let affected = diesel::update(dsl::users.filter(dsl::id.eq(user_id)))
            .set(dsl::password_hash.eq(new_hash))
            .execute(conn.as_mut())?;

        if affected == 1 {
            Ok(())
        } else if affected == 0 {
            Err(RepoError::NotFound)
        } else {
            Err(RepoError::Db(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::Unknown,
                Box::new(format!("updated {} rows for id={}", affected, user_id)),
            )))
        }
    }

    fn insert_user(&mut self, new: &NewUser) -> Result<i32, RepoError> {
        let mut conn = self.get_conn()?;
        let id = conn
            .as_mut()
            .transaction::<i32, diesel::result::Error, _>(|conn| {
                let res: User = diesel::insert_into(schema::users::table)
                    .values(new)
                    .get_result(conn)?;
                Ok(res.id)
            })
            .map_err(map_unique_violation)?;

        Ok(id)
    }

    fn update_user(&mut self, user_data: &NewUser) -> Result<bool, RepoError> {
        use crate::schema::users::dsl;
        let mut conn = self.get_conn()?;
        let user_id_value = user_data
            .id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let result = diesel::update(dsl::users.filter(dsl::id.eq(user_id_value)))
            .set((
                dsl::name.eq(&user_data.name),
                dsl::username.eq(&user_data.username),
                dsl::email.eq(&user_data.email),
                dsl::password_hash.eq(&user_data.password_hash),
                dsl::is_admin.eq(user_data.is_admin),
            ))
            .execute(conn.as_mut())
            .map_err(map_unique_violation)?;
        Ok(result > 0)
    }

    fn update_user_without_password(&mut self, user_data: &UpdateUser) -> Result<bool, RepoError> {
        use crate::schema::users::dsl;
        let mut conn = self.get_conn()?;
        let user_id_value = user_data
            .id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let result = diesel::update(dsl::users.filter(dsl::id.eq(user_id_value)))
            .set((
                dsl::name.eq(&user_data.name),
                dsl::username.eq(&user_data.username),
                dsl::email.eq(&user_data.email),
                dsl::is_admin.eq(user_data.is_admin),
            ))
            .execute(conn.as_mut())
            .map_err(map_unique_violation)?;
        Ok(result > 0)
    }

    fn delete_user(&mut self, user_id: i32) -> Result<User, RepoError> {
        use crate::schema::users::dsl;
        let mut conn = self.get_conn()?;
        let user = dsl::users
            .filter(dsl::id.eq(user_id))
            .get_result::<User>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        diesel::delete(dsl::users.filter(dsl::id.eq(user_id))).execute(conn.as_mut())?;
        Ok(user)
    }

    fn get_user_by_email(&self, email: &str) -> Result<Option<User>, RepoError> {
        self.db_get_user_by_email(email)
    }

    fn set_user_email_verified(&mut self, user_id: i32, verified: bool) -> Result<(), RepoError> {
        self.db_set_user_email_verified(user_id, verified)
    }
}

impl ApiTokensRepository for DieselRepo {
    fn get_user_by_token_hash(&self, token_hash: &str) -> Result<(User, Option<i32>), RepoError> {
        use schema::user_api_tokens::dsl as tok_dsl;
        let mut conn = self.get_conn()?;
        let row: (User, Option<i32>) = schema::user_api_tokens::table
            .inner_join(
                schema::users::table.on(schema::user_api_tokens::user_id.eq(schema::users::id)),
            )
            .filter(tok_dsl::token_hash.eq(token_hash))
            .select((
                schema::users::all_columns,
                schema::user_api_tokens::project_id,
            ))
            .first(conn.as_mut())
            .map_err(|e: diesel::result::Error| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })?;
        Ok(row)
    }

    fn update_api_token_last_used_at(&mut self, token_hash: &str) -> Result<(), RepoError> {
        use schema::user_api_tokens::dsl;
        let mut conn = self.get_conn()?;
        let now = chrono::Utc::now().naive_utc();
        diesel::update(dsl::user_api_tokens.filter(dsl::token_hash.eq(token_hash)))
            .set(dsl::last_used_at.eq(now))
            .execute(conn.as_mut())?;
        Ok(())
    }
}
