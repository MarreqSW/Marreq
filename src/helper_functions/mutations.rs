use crate::models::*;
use diesel::dsl::now;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use std::error::Error;

pub fn insert_new_requirement(conn: &mut PgConnection, new: &NewRequirement)
            -> Result<i32, Box<dyn Error>> {
    let res: Requirement = diesel::insert_into(crate::schema::requirements::table)
        .values(new)
        .get_result(conn)?;
    Ok(res.req_id)
}

pub fn edit_requirement(
    conn: &mut PgConnection,
    new: &NewRequirement,
) -> Result<bool, Box<dyn Error>> {
    use crate::schema::requirements::dsl::*;

    let id = new.req_id.unwrap_or(0);

    diesel::update(requirements)
        .filter(req_id.eq(id))
        .set(new)
        .execute(conn)?;

    Ok(true)
}

pub fn delete_requirement(conn: &mut PgConnection, id: &i32) -> Result<bool, Box<dyn Error>> {
    use crate::schema::requirements::dsl::*;

    let deleted = diesel::delete(requirements.filter(req_id.eq(id)))
        .execute(conn)?;

    Ok(deleted > 0)
}

pub fn delete_test(conn: &mut PgConnection, id: &i32) -> Result<bool, Box<dyn Error>> {
    use crate::schema::tests::dsl::*;

    let deleted = diesel::delete(tests.filter(test_id.eq(id)))
        .execute(conn)?;

    Ok(deleted > 0)
}

pub fn delete_user(conn: &mut PgConnection, id: &i32) -> Result<bool, Box<dyn Error>> {
    use crate::schema::users::dsl::*;

    let deleted = diesel::delete(users.filter(user_id.eq(id)))
        .execute(conn)?;

    Ok(deleted > 0)
}

pub fn insert_new_test(conn: &mut PgConnection, new: &NewTest) -> Result<i32, Box<dyn Error>> {
    let res: Test = diesel::insert_into(crate::schema::tests::table)
        .values(new)
        .get_result(conn)?;
    Ok(res.test_id)
}

pub fn edit_test(conn: &mut PgConnection, new: &NewTest) -> Result<bool, Box<dyn Error>> {
    use crate::schema::tests::dsl::*;

    let test_id_value = new.test_id.unwrap_or(0);
    if test_id_value == 0 {
        return Err("Test ID is required for editing".into());
    }

    let updated = diesel::update(tests.filter(test_id.eq(test_id_value)))
        .set((
            test_name.eq(&new.test_name),
            test_description.eq(&new.test_description),
            test_source.eq(&new.test_source),
            test_status.eq(&new.test_status),
            test_parent.eq(&new.test_parent),
        ))
        .execute(conn)?;

    Ok(updated > 0)
}

pub fn insert_new_matrix_item(
    conn: &mut PgConnection,
    new: &NewMatrix,
) -> Result<(), Box<dyn Error>> {
    #[cfg(debug_assertions)]
    println!("Inserting, ({}, {})", new.matrix_req_id, new.matrix_test_id);
    diesel::insert_into(crate::schema::matrix::table)
        .values(new)
        .execute(conn)?;
    Ok(())
}

pub fn update_test_requirement_links(
    conn: &mut PgConnection,
    test_id: i32,
    requirement_ids: &[i32],
) -> Result<(), Box<dyn Error>> {
    use crate::schema::matrix::dsl::*;

    diesel::delete(matrix.filter(matrix_test_id.eq(test_id)))
        .execute(conn)?;

    for req_id in requirement_ids {
        let matrix_item = NewMatrix {
            matrix_req_id: *req_id,
            matrix_test_id: test_id,
            project_id: 1,
        };
        insert_new_matrix_item(conn, &matrix_item)?;
    }

    Ok(())
}

pub fn insert_new_user(conn: &mut PgConnection, new: &NewUser) -> Result<i32, Box<dyn Error>> {
    let a: User = diesel::insert_into(crate::schema::users::table)
        .values(new)
        .get_result(conn)?;
    #[cfg(debug_assertions)]
    println!("New user id {}", a.user_id);
    Ok(a.user_id)
}

pub fn update_user(conn: &mut PgConnection, user_data: &NewUser) -> Result<bool, Box<dyn Error>> {
    use crate::schema::users::dsl::*;

    let user_id_value = user_data.user_id.ok_or("User ID is required")?;

    let result = diesel::update(users.filter(user_id.eq(user_id_value)))
        .set((
            user_name.eq(&user_data.user_name),
            user_username.eq(&user_data.user_username),
            user_email.eq(&user_data.user_email),
            user_level.eq(user_data.user_level),
        ))
        .execute(conn)?;

    Ok(result > 0)
}

pub fn update_user_without_password(conn: &mut PgConnection, user_data: &crate::models::UpdateUser) -> Result<bool, Box<dyn Error>> {
    use crate::schema::users::dsl::*;

    let user_id_value = user_data.user_id.ok_or("User ID is required")?;

    let result = diesel::update(users.filter(user_id.eq(user_id_value)))
        .set((
            user_name.eq(&user_data.user_name),
            user_username.eq(&user_data.user_username),
            user_email.eq(&user_data.user_email),
            user_level.eq(user_data.user_level),
        ))
        .execute(conn)?;

    Ok(result > 0)
}

pub fn update_requirement(conn: &mut PgConnection, req: i32) -> Result<(), Box<dyn Error>> {
    use crate::schema::requirements::dsl::*;

    diesel::update(requirements)
        .filter(req_id.eq(req))
        .set(req_update_date.eq(now))
        .execute(conn)?;

    Ok(())
}

pub fn create_test(conn: &mut PgConnection, new: &NewTest)
            -> Result<i32, Box<dyn Error>> {
    let res: Test = diesel::insert_into(crate::schema::tests::table)
        .values(new)
        .get_result(conn)?;
    Ok(res.test_id)
}

pub fn create_status(conn: &mut PgConnection, new: &NewStatus)
        -> Result<i32, Box<dyn Error>> {
    let res: Status = diesel::insert_into(crate::schema::status::table)
        .values(new)
        .get_result(conn)?;
    Ok(res.st_id)
}

pub fn create_user(conn: &mut PgConnection, new: &NewUser) -> Result<i32, Box<dyn Error>> {
    let res: User = diesel::insert_into(crate::schema::users::table)
        .values(new)
        .get_result(conn)?;
    Ok(res.user_id)
}

pub fn insert_new_category(conn: &mut PgConnection, new: &NewCategory) -> Result<i32, Box<dyn Error>> {
    use crate::schema::categories::dsl::*;

    let result = diesel::insert_into(categories)
        .values(new)
        .get_result::<Category>(conn)?;
    Ok(result.cat_id)
}

pub fn edit_category(conn: &mut PgConnection, new: &NewCategory) -> Result<bool, Box<dyn Error>> {
    use crate::schema::categories::dsl::*;

    let category_id = new.cat_id.unwrap_or(0);
    if category_id == 0 {
        return Err("Category ID is required for editing".into());
    }

    let updated = diesel::update(categories.filter(cat_id.eq(category_id)))
        .set((
            cat_title.eq(&new.cat_title),
            cat_description.eq(&new.cat_description),
            cat_tag.eq(&new.cat_tag),
        ))
        .execute(conn)?;

    Ok(updated > 0)
}

pub fn delete_category(conn: &mut PgConnection, id: &i32) -> Result<bool, Box<dyn Error>> {
    use crate::schema::categories::dsl::*;

    let deleted = diesel::delete(categories.filter(cat_id.eq(id)))
        .execute(conn)?;

    Ok(deleted > 0)
}

pub fn insert_new_project(conn: &mut PgConnection, new: &NewProject) -> Result<i32, Box<dyn Error>> {
    use crate::schema::projects::dsl::*;

    let result = diesel::insert_into(projects)
        .values(new)
        .get_result::<Project>(conn)?;
    Ok(result.project_id)
}

pub fn edit_project(conn: &mut PgConnection, project_id_param: i32, update: &UpdateProject) -> Result<bool, Box<dyn Error>> {
    use crate::schema::projects::dsl::*;

    let updated = diesel::update(projects.filter(project_id.eq(project_id_param)))
        .set((
            project_name.eq(&update.project_name),
            project_description.eq(&update.project_description),
            project_status.eq(&update.project_status),
            project_owner_id.eq(&update.project_owner_id),
            project_update_date.eq(chrono::Utc::now().naive_utc()),
        ))
        .execute(conn)?;

    Ok(updated > 0)
}

pub fn delete_project(conn: &mut PgConnection, project_id_param: &i32) -> Result<bool, Box<dyn Error>> {
    use crate::schema::projects::dsl::*;

    let deleted = diesel::delete(projects.filter(project_id.eq(project_id_param)))
        .execute(conn)?;

    Ok(deleted > 0)
}

pub fn insert_new_applicability(conn: &mut PgConnection, new: &NewApplicability) -> Result<i32, Box<dyn Error>> {
    use crate::schema::applicability::dsl::*;

    let result = diesel::insert_into(applicability)
        .values(new)
        .get_result::<Applicability>(conn)?;
    Ok(result.app_id)
}

pub fn edit_applicability(conn: &mut PgConnection, new: &NewApplicability) -> Result<bool, Box<dyn Error>> {
    use crate::schema::applicability::dsl::*;

    let applicability_id = new.app_id.unwrap_or(0);
    if applicability_id == 0 {
        return Err("Applicability ID is required for editing".into());
    }

    let updated = diesel::update(applicability.filter(app_id.eq(applicability_id)))
        .set((
            app_title.eq(&new.app_title),
            app_description.eq(&new.app_description),
            app_tag.eq(&new.app_tag),
        ))
        .execute(conn)?;

    Ok(updated > 0)
}

pub fn delete_applicability(conn: &mut PgConnection, id: &i32) -> Result<bool, Box<dyn Error>> {
    use crate::schema::applicability::dsl::*;

    let deleted = diesel::delete(applicability.filter(app_id.eq(id)))
        .execute(conn)?;

    Ok(deleted > 0)
}
