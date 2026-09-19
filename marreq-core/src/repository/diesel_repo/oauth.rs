use chrono::NaiveDateTime;
use diesel::prelude::*;

use super::{map_db_error, DieselRepo};
use crate::models::*;
use crate::repository::{DelegatedOAuthRepository, RepoError};
use crate::schema::{
    oauth_access_tokens, oauth_authorization_codes, oauth_clients, oauth_grants,
    oauth_refresh_tokens, users,
};

impl DelegatedOAuthRepository for DieselRepo {
    fn insert_oauth_client(&mut self, value: &NewOAuthClient) -> Result<(), RepoError> {
        diesel::insert_into(oauth_clients::table)
            .values(value)
            .execute(&mut *self.get_conn()?)
            .map(|_| ())
            .map_err(map_db_error)
    }

    fn get_oauth_client(&self, id: &str) -> Result<OAuthClient, RepoError> {
        oauth_clients::table
            .find(id)
            .select(OAuthClient::as_select())
            .first(&mut *self.get_conn()?)
            .map_err(Into::into)
    }

    fn upsert_oauth_grant(&mut self, value: &NewOAuthGrant) -> Result<OAuthGrant, RepoError> {
        diesel::insert_into(oauth_grants::table)
            .values(value)
            .on_conflict((
                oauth_grants::user_id,
                oauth_grants::client_id,
                oauth_grants::resource,
            ))
            .do_update()
            .set((
                oauth_grants::scopes.eq(&value.scopes),
                oauth_grants::updated_at.eq(diesel::dsl::now),
                oauth_grants::revoked_at.eq::<Option<NaiveDateTime>>(None),
            ))
            .returning(OAuthGrant::as_returning())
            .get_result(&mut *self.get_conn()?)
            .map_err(map_db_error)
    }

    fn list_oauth_grants(&self, owner: i32) -> Result<Vec<(OAuthGrant, OAuthClient)>, RepoError> {
        oauth_grants::table
            .inner_join(oauth_clients::table)
            .filter(
                oauth_grants::user_id
                    .eq(owner)
                    .and(oauth_grants::revoked_at.is_null()),
            )
            .select((OAuthGrant::as_select(), OAuthClient::as_select()))
            .order(oauth_grants::created_at.desc())
            .load(&mut *self.get_conn()?)
            .map_err(Into::into)
    }

    fn revoke_oauth_grant(
        &mut self,
        id: i32,
        owner: i32,
        now: NaiveDateTime,
    ) -> Result<bool, RepoError> {
        let mut conn = self.get_conn()?;
        conn.transaction(|conn| {
            let count = diesel::update(
                oauth_grants::table.filter(
                    oauth_grants::id
                        .eq(id)
                        .and(oauth_grants::user_id.eq(owner))
                        .and(oauth_grants::revoked_at.is_null()),
                ),
            )
            .set(oauth_grants::revoked_at.eq(now))
            .execute(conn)?;
            if count > 0 {
                diesel::delete(
                    oauth_access_tokens::table.filter(oauth_access_tokens::grant_id.eq(id)),
                )
                .execute(conn)?;
                diesel::update(
                    oauth_refresh_tokens::table.filter(
                        oauth_refresh_tokens::grant_id
                            .eq(id)
                            .and(oauth_refresh_tokens::revoked_at.is_null()),
                    ),
                )
                .set(oauth_refresh_tokens::revoked_at.eq(now))
                .execute(conn)?;
            }
            Ok(count > 0)
        })
        .map_err(map_db_error)
    }

    fn insert_oauth_code(&mut self, value: &NewOAuthAuthorizationCode) -> Result<(), RepoError> {
        diesel::insert_into(oauth_authorization_codes::table)
            .values(value)
            .execute(&mut *self.get_conn()?)
            .map(|_| ())
            .map_err(map_db_error)
    }
    fn get_oauth_code(&self, hash: &str) -> Result<OAuthAuthorizationCode, RepoError> {
        oauth_authorization_codes::table
            .find(hash)
            .select(OAuthAuthorizationCode::as_select())
            .first(&mut *self.get_conn()?)
            .map_err(Into::into)
    }
    fn consume_oauth_code(&mut self, hash: &str, now: NaiveDateTime) -> Result<bool, RepoError> {
        diesel::update(
            oauth_authorization_codes::table.filter(
                oauth_authorization_codes::code_hash
                    .eq(hash)
                    .and(oauth_authorization_codes::used_at.is_null())
                    .and(oauth_authorization_codes::expires_at.gt(now)),
            ),
        )
        .set(oauth_authorization_codes::used_at.eq(now))
        .execute(&mut *self.get_conn()?)
        .map(|n| n == 1)
        .map_err(Into::into)
    }
    fn insert_oauth_tokens(
        &mut self,
        access: &NewOAuthAccessToken,
        refresh: &NewOAuthRefreshToken,
    ) -> Result<(), RepoError> {
        let mut conn = self.get_conn()?;
        conn.transaction(|conn| {
            diesel::insert_into(oauth_access_tokens::table)
                .values(access)
                .execute(conn)?;
            diesel::insert_into(oauth_refresh_tokens::table)
                .values(refresh)
                .execute(conn)?;
            Ok(())
        })
        .map_err(map_db_error)
    }
    fn get_oauth_access_token(
        &self,
        hash: &str,
    ) -> Result<(OAuthAccessToken, OAuthGrant, User), RepoError> {
        oauth_access_tokens::table
            .inner_join(oauth_grants::table.inner_join(users::table))
            .filter(oauth_access_tokens::token_hash.eq(hash))
            .select((
                OAuthAccessToken::as_select(),
                OAuthGrant::as_select(),
                users::all_columns,
            ))
            .first(&mut *self.get_conn()?)
            .map_err(Into::into)
    }
    fn get_oauth_refresh_token(
        &self,
        hash: &str,
    ) -> Result<(OAuthRefreshToken, OAuthGrant), RepoError> {
        oauth_refresh_tokens::table
            .inner_join(oauth_grants::table)
            .filter(oauth_refresh_tokens::token_hash.eq(hash))
            .select((OAuthRefreshToken::as_select(), OAuthGrant::as_select()))
            .first(&mut *self.get_conn()?)
            .map_err(Into::into)
    }
    fn rotate_oauth_refresh_token(
        &mut self,
        old: &str,
        access: &NewOAuthAccessToken,
        refresh: &NewOAuthRefreshToken,
        now: NaiveDateTime,
    ) -> Result<bool, RepoError> {
        let mut conn = self.get_conn()?;
        conn.transaction(|conn| {
            let count = diesel::update(
                oauth_refresh_tokens::table.filter(
                    oauth_refresh_tokens::token_hash
                        .eq(old)
                        .and(oauth_refresh_tokens::used_at.is_null())
                        .and(oauth_refresh_tokens::revoked_at.is_null())
                        .and(oauth_refresh_tokens::expires_at.gt(now)),
                ),
            )
            .set((
                oauth_refresh_tokens::used_at.eq(now),
                oauth_refresh_tokens::replaced_by_hash.eq(Some(refresh.token_hash.clone())),
            ))
            .execute(conn)?;
            if count != 1 {
                return Ok(false);
            }
            diesel::insert_into(oauth_access_tokens::table)
                .values(access)
                .execute(conn)?;
            diesel::insert_into(oauth_refresh_tokens::table)
                .values(refresh)
                .execute(conn)?;
            Ok(true)
        })
        .map_err(map_db_error)
    }
    fn revoke_oauth_refresh_family(
        &mut self,
        family: &str,
        now: NaiveDateTime,
    ) -> Result<(), RepoError> {
        diesel::update(
            oauth_refresh_tokens::table.filter(
                oauth_refresh_tokens::family_id
                    .eq(family)
                    .and(oauth_refresh_tokens::revoked_at.is_null()),
            ),
        )
        .set(oauth_refresh_tokens::revoked_at.eq(now))
        .execute(&mut *self.get_conn()?)
        .map(|_| ())
        .map_err(Into::into)
    }
}
