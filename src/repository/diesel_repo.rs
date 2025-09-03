use diesel::prelude::*;
use super::errors::RepoError;
use crate::repository::{LookupRepository, UserRepository};
use crate::models::*;
use crate::schema;

pub struct DieselRepo {
    // TODO: move db connection pool here
}

impl DieselRepo {
    pub fn new() -> Self { Self {} }
}

impl UserRepository for DieselRepo {

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

impl LookupRepository for DieselRepo {
    fn get_status_all(&self) -> Result<Vec<Status>, RepoError> {
        use schema::status::dsl::*;
        let mut conn = crate::db::get_connection_pooled_safe()
            .map_err(|e| RepoError::Pool(e.to_string()))?;
        status
            .order(st_id)
            .load::<Status>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_status_by_id(&self, id: i32) -> Result<Status, RepoError> {
        use schema::status::dsl::*;
        let mut conn = crate::db::get_connection_pooled_safe()
            .map_err(|e| RepoError::Pool(e.to_string()))?;
        status
            .filter(st_id.eq(id))
            .get_result(conn.as_mut())
            .map_err(|e| if e == diesel::result::Error::NotFound {
                RepoError::NotFound
            } else {
                e.into()
            })
    }

    fn get_categories_all(&self) -> Result<Vec<Category>, RepoError> {
        use schema::categories::dsl::*;
        let mut conn = crate::db::get_connection_pooled_safe()
            .map_err(|e| RepoError::Pool(e.to_string()))?;
        categories
            .order(cat_id)
            .load::<Category>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_category_by_id(&self, id: i32) -> Result<Category, RepoError> {
        use schema::categories::dsl::*;
        let mut conn = crate::db::get_connection_pooled_safe()
            .map_err(|e| RepoError::Pool(e.to_string()))?;
        categories
            .filter(cat_id.eq(id))
            .get_result(conn.as_mut())
            .map_err(|e| if e == diesel::result::Error::NotFound {
                RepoError::NotFound
            } else {
                e.into()
            })
    }

    fn get_applicability_all(&self) -> Result<Vec<Applicability>, RepoError> {
        use schema::applicability::dsl::*;
        let mut conn = crate::db::get_connection_pooled_safe()
            .map_err(|e| RepoError::Pool(e.to_string()))?;
        applicability
            .order(app_id)
            .load::<Applicability>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_applicability_by_id(&self, id: i32) -> Result<Applicability, RepoError> {
        use schema::applicability::dsl::*;
        let mut conn = crate::db::get_connection_pooled_safe()
            .map_err(|e| RepoError::Pool(e.to_string()))?;
        applicability
            .filter(app_id.eq(id))
            .get_result(conn.as_mut())
            .map_err(|e| if e == diesel::result::Error::NotFound {
                RepoError::NotFound
            } else {
                e.into()
            })
    }

    fn get_verification_all(&self) -> Result<Vec<Verification>, RepoError> {
        use schema::verification::dsl::*;
        let mut conn = crate::db::get_connection_pooled_safe()
            .map_err(|e| RepoError::Pool(e.to_string()))?;
        verification
            .order(verification_id)
            .load::<Verification>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_verification_by_id(&self, id: i32) -> Result<Verification, RepoError> {
        use schema::verification::dsl::*;
        let mut conn = crate::db::get_connection_pooled_safe()
            .map_err(|e| RepoError::Pool(e.to_string()))?;
        verification
            .filter(verification_id.eq(id))
            .get_result(conn.as_mut())
            .map_err(|e| if e == diesel::result::Error::NotFound {
                RepoError::NotFound
            } else {
                e.into()
            })
    }
}
