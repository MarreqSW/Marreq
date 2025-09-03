use diesel::prelude::*;
use super::errors::RepoError;
use crate::models::*;
use crate::schema;
use crate::repository::{
    LookupRepository, MatrixRepository, ProjectsRepository, RequirementsRepository,
    TestsRepository, UserRepository,
};

pub struct DieselRepo {
    // TODO: move db connection pool here
}

impl DieselRepo {
    pub fn new() -> Self { Self {} }
}

impl UserRepository for DieselRepo {

    fn get_users_all(&self) -> Result<Vec<User>, RepoError> {
        use schema::users::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        users
            .order(user_id)
            .load::<User>(conn.as_mut())
            .map_err(|e| e.into())
    }

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


    fn insert_new_user(&mut self, new: &NewUser) -> Result<i32, RepoError> {
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let res: User =
            diesel::insert_into(schema::users::table).values(new).get_result(conn.as_mut())?;
        Ok(res.user_id)
    }

    fn update_user(&mut self, user_data: &NewUser) -> Result<bool, RepoError> {
        use crate::schema::users::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let user_id_value = user_data
            .user_id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let result = diesel::update(users.filter(user_id.eq(user_id_value)))
            .set((
                user_name.eq(&user_data.user_name),
                user_username.eq(&user_data.user_username),
                user_email.eq(&user_data.user_email),
                user_level.eq(user_data.user_level),
                user_password.eq(&user_data.user_password),
            ))
            .execute(conn.as_mut())?;
        Ok(result > 0)
    }

    fn update_user_without_password(
        &mut self,
        user_data: &UpdateUser,
    ) -> Result<bool, RepoError> {
        use crate::schema::users::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let user_id_value = user_data
            .user_id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let result = diesel::update(users.filter(user_id.eq(user_id_value)))
            .set((
                user_name.eq(&user_data.user_name),
                user_username.eq(&user_data.user_username),
                user_email.eq(&user_data.user_email),
                user_level.eq(user_data.user_level),
            ))
            .execute(conn.as_mut())?;
        Ok(result > 0)
    }

    fn delete_user(&mut self, id: i32) -> Result<bool, RepoError> {
        use crate::schema::users::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let deleted = diesel::delete(users.filter(user_id.eq(id))).execute(conn.as_mut())?;
        Ok(deleted > 0)
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

    fn get_categories_by_project(&self, pid: i32) -> Result<Vec<Category>, RepoError> {
        use schema::categories::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        categories
            .filter(project_id.eq(pid))
            .load::<Category>(conn.as_mut())
            .map_err(|e| e.into())
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

    fn get_applicability_by_project(&self, pid: i32) -> Result<Vec<Applicability>, RepoError> {
        use schema::applicability::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        applicability
            .filter(project_id.eq(pid))
            .load::<Applicability>(conn.as_mut())
            .map_err(|e| e.into())
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

    fn get_verification_by_project(&self, pid: i32) -> Result<Vec<Verification>, RepoError> {
        use schema::verification::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        verification
            .filter(project_id.eq(pid))
            .order(verification_id)
            .load::<Verification>(conn.as_mut())
            .map_err(|e| e.into())
    }

        fn insert_new_category(&mut self, new: &NewCategory) -> Result<i32, RepoError> {
        use schema::categories::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let result = diesel::insert_into(categories)
            .values(new)
            .get_result::<Category>(conn.as_mut())?;
        Ok(result.cat_id)
    }

    fn edit_category(&mut self, new: &NewCategory) -> Result<bool, RepoError> {
        use schema::categories::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let category_id = new
            .cat_id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let updated = diesel::update(categories.filter(cat_id.eq(category_id)))
            .set((
                cat_title.eq(&new.cat_title),
                cat_description.eq(&new.cat_description),
                cat_tag.eq(&new.cat_tag),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_category(&mut self, id: i32) -> Result<bool, RepoError> {
        use schema::categories::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let deleted = diesel::delete(categories.filter(cat_id.eq(id))).execute(conn.as_mut())?;
        Ok(deleted > 0)
    }

    fn insert_new_applicability(&mut self, new: &NewApplicability) -> Result<i32, RepoError> {
        use schema::applicability::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let result = diesel::insert_into(applicability)
            .values(new)
            .get_result::<Applicability>(conn.as_mut())?;
        Ok(result.app_id)
    }

    fn edit_applicability(&mut self, new: &NewApplicability) -> Result<bool, RepoError> {
        use schema::applicability::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let app_id_val = new
            .app_id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let updated = diesel::update(applicability.filter(app_id.eq(app_id_val)))
            .set((
                app_title.eq(&new.app_title),
                app_description.eq(&new.app_description),
                app_tag.eq(&new.app_tag),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_applicability(&mut self, id: i32) -> Result<bool, RepoError> {
        use schema::applicability::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let deleted = diesel::delete(applicability.filter(app_id.eq(id))).execute(conn.as_mut())?;
        Ok(deleted > 0)
    }

    fn create_status(&mut self, new: &NewStatus) -> Result<i32, RepoError> {
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let res: Status =
            diesel::insert_into(schema::status::table).values(new).get_result(conn.as_mut())?;
        Ok(res.st_id)
    }
}

impl RequirementsRepository for DieselRepo {

    fn get_requirement_by_id(&self, id: i32) -> Result<Requirement, RepoError> {
        use schema::requirements::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        requirements
            .filter(req_id.eq(id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_requirements_all(&self) -> Result<Vec<Requirement>, RepoError> {
        use schema::requirements::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        requirements
            .order(req_id)
            .load::<Requirement>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_requirements_by_project(&self, project: i32) -> Result<Vec<Requirement>, RepoError> {
        use schema::requirements::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        requirements
            .filter(schema::requirements::project_id.eq(project))
            .load::<Requirement>(conn.as_mut())
            .map_err(|e| e.into())
    }

        fn insert_new_requirement(&mut self, new: &NewRequirement) -> Result<i32, RepoError> {
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let res: Requirement = diesel::insert_into(schema::requirements::table)
            .values(new)
            .get_result(conn.as_mut())?;
        Ok(res.req_id)
    }

    fn edit_requirement(&mut self, new: &NewRequirement) -> Result<bool, RepoError> {
        use crate::schema::requirements::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let id_val = new
            .req_id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        diesel::update(requirements.filter(req_id.eq(id_val)))
            .set(new)
            .execute(conn.as_mut())
            .map(|_| true)
            .map_err(|e| e.into())
    }

    fn delete_requirement(&mut self, id: i32) -> Result<bool, RepoError> {
        use crate::schema::requirements::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let deleted =
            diesel::delete(requirements.filter(req_id.eq(id))).execute(conn.as_mut())?;
        Ok(deleted > 0)
    }

    fn update_requirement(&mut self, req: i32) -> Result<(), RepoError> {
        use crate::schema::requirements::dsl::*;
        use diesel::dsl::now;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        diesel::update(requirements)
            .filter(req_id.eq(req))
            .set(req_update_date.eq(now))
            .execute(conn.as_mut())?;
        Ok(())
    }
}

impl TestsRepository for DieselRepo {
    fn get_test_by_id(&self, id: i32) -> Result<Test, RepoError> {
        use schema::tests::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        tests
            .filter(test_id.eq(id))
            .get_result(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn get_tests_all(&self) -> Result<Vec<Test>, RepoError> {
        use schema::tests::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        tests
            .order(test_id)
            .load::<Test>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_tests_by_project(&self, project: i32) -> Result<Vec<Test>, RepoError> {
        use schema::tests::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        tests
            .filter(schema::tests::project_id.eq(project))
            .load::<Test>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_requirements_for_test(&self, tid: i32) -> Result<Vec<Requirement>, RepoError> {
        use schema::matrix::dsl::*;
        use schema::requirements::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        matrix
            .filter(matrix_test_id.eq(tid))
            .inner_join(requirements.on(matrix_req_id.eq(req_id)))
            .select((
                req_id,
                req_title,
                req_description,
                req_verification,
                req_current_status,
                req_author,
                req_reviewer,
                req_link,
                req_reference,
                req_category,
                req_parent,
                req_creation_date,
                req_update_date,
                req_deadline_date,
                req_applicability,
                req_justification,
                schema::requirements::project_id,
            ))
            .load::<Requirement>(conn.as_mut())
            .map_err(|e| e.into())
    }

       fn insert_new_test(&mut self, new: &NewTest) -> Result<i32, RepoError> {
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let res: Test =
            diesel::insert_into(schema::tests::table).values(new).get_result(conn.as_mut())?;
        Ok(res.test_id)
    }

    fn edit_test(&mut self, new: &NewTest) -> Result<bool, RepoError> {
        use crate::schema::tests::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let test_id_value = new
            .test_id
            .ok_or(RepoError::Db(diesel::result::Error::NotFound))?;
        let updated = diesel::update(tests.filter(test_id.eq(test_id_value)))
            .set((
                test_name.eq(&new.test_name),
                test_description.eq(&new.test_description),
                test_source.eq(&new.test_source),
                test_status.eq(&new.test_status),
                test_parent.eq(&new.test_parent),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_test(&mut self, id: i32) -> Result<bool, RepoError> {
        use crate::schema::tests::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let deleted = diesel::delete(tests.filter(test_id.eq(id))).execute(conn.as_mut())?;
        Ok(deleted > 0)
    }

    fn update_test_requirement_links(
        &mut self,
        test_id_val: i32,
        requirement_ids: &[i32],
    ) -> Result<(), RepoError> {
        use schema::matrix::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        diesel::delete(matrix.filter(matrix_test_id.eq(test_id_val))).execute(conn.as_mut())?;
        for req_id in requirement_ids {
            let matrix_item = NewMatrix {
                matrix_req_id: *req_id,
                matrix_test_id: test_id_val,
                project_id: 1,
            };
            diesel::insert_into(schema::matrix::table)
                .values(&matrix_item)
                .execute(conn.as_mut())?;
        }
        Ok(())
    }
}

impl ProjectsRepository for DieselRepo {
    fn get_projects_all(&self) -> Result<Vec<Project>, RepoError> {
        use schema::projects::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        projects
            .load::<Project>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn get_project_by_id(&self, id: i32) -> Result<Project, RepoError> {
        use schema::projects::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        projects
            .filter(project_id.eq(id))
            .first::<Project>(conn.as_mut())
            .map_err(|e| {
                if e == diesel::result::Error::NotFound {
                    RepoError::NotFound
                } else {
                    e.into()
                }
            })
    }

    fn insert_new_project(&mut self, new: &NewProject) -> Result<i32, RepoError> {
        use schema::projects::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let result = diesel::insert_into(projects)
            .values(new)
            .get_result::<Project>(conn.as_mut())?;
        Ok(result.project_id)
    }

    fn edit_project(
        &mut self,
        project_id_param: i32,
        update: &UpdateProject,
    ) -> Result<bool, RepoError> {
        use schema::projects::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let updated = diesel::update(projects.filter(project_id.eq(project_id_param)))
            .set((
                project_name.eq(&update.project_name),
                project_description.eq(&update.project_description),
                project_status.eq(&update.project_status),
                project_owner_id.eq(&update.project_owner_id),
                project_update_date.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(conn.as_mut())?;
        Ok(updated > 0)
    }

    fn delete_project(&mut self, project_id_param: i32) -> Result<bool, RepoError> {
        use schema::projects::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        let deleted =
            diesel::delete(projects.filter(project_id.eq(project_id_param))).execute(conn.as_mut())?;
        Ok(deleted > 0)
    }
}

impl MatrixRepository for DieselRepo {

    fn get_matrix_by_project(&self, pid: i32) -> Result<Vec<Matrix>, RepoError> {
        use schema::matrix::dsl::*;
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        matrix
            .filter(project_id.eq(pid))
            .load::<Matrix>(conn.as_mut())
            .map_err(|e| e.into())
    }

    fn insert_new_matrix_item(&mut self, new: &NewMatrix) -> Result<(), RepoError> {
        let mut conn =
            crate::db::get_connection_pooled_safe().map_err(|e| RepoError::Pool(e.to_string()))?;
        diesel::insert_into(schema::matrix::table)
            .values(new)
            .execute(conn.as_mut())?;
        Ok(())
    }
}
