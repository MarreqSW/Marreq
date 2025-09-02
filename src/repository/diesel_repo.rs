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


}
