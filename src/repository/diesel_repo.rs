use diesel::prelude::*;
use super::errors::RepoError;
use crate::repository::Repository;
use crate::models::*;
use crate::schema;

pub struct DieselRepo {
    // TODO: move db connection pool here
}

impl Repository for DieselRepo {

    fn get_user_by_id(&self, idv: i32) -> Result<User, RepoError> {
        use schema::users::dsl::*;
        let mut conn = crate::db::get_connection_pooled_safe()
            .map_err(|e| RepoError::Pool(e.to_string()))?;

        users
            .filter(user_id.eq(idv))
            .first::<User>(conn.as_mut()) // <-- use inner PgConnection
            .map_err(|e| if e == diesel::result::Error::NotFound {
                RepoError::NotFound
            } else {
                e.into()
            })
    }

    fn get_user_by_username(&self, uname: &str) -> Result<Option<User>, RepoError> {
        use crate::schema::users::dsl::*;
        let mut conn = crate::db::get_connection_pooled_safe()
            .map_err(|e| RepoError::Pool(e.to_string()))?;

        users
            .filter(user_username.eq(uname))
            .first::<User>(conn.as_mut())
            .optional()
            .map_err(|e| e.into())
    }

    fn update_user_password(&mut self, id: i32, new_hash: &str) -> Result<(), RepoError> {
        use crate::schema::users::dsl::*;
        let mut conn = crate::db::get_connection_pooled_safe()
            .map_err(|e| RepoError::Pool(e.to_string()))?;

        let affected = diesel::update(users.filter(user_id.eq(id)))
            .set(user_password.eq(new_hash))
            .execute(conn.as_mut())?;

        if affected == 1 {
            Ok(())
        } else if affected == 0 {
            Err(RepoError::NotFound)
        } else {
            Err(RepoError::Db(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::Unknown,
                Box::new(format!("updated {} rows for user_id={}", affected, id)),
            )))
        }
    }
}
