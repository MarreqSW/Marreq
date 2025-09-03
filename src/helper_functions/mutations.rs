use crate::models::*;
use crate::repository::{
    DieselRepo, LookupRepository, MatrixRepository, ProjectsRepository, RequirementsRepository,
    TestsRepository, UserRepository,
};
use diesel::pg::PgConnection;
use std::error::Error;

pub fn insert_new_requirement(
    _conn: &mut PgConnection,
    new: &NewRequirement,
) -> Result<i32, Box<dyn Error>> {
    DieselRepo::new()
        .insert_new_requirement(new)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn edit_requirement(
    _conn: &mut PgConnection,
    new: &NewRequirement,
) -> Result<bool, Box<dyn Error>> {
    DieselRepo::new()
        .edit_requirement(new)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn delete_requirement(
    _conn: &mut PgConnection,
    id: &i32,
) -> Result<bool, Box<dyn Error>> {
    DieselRepo::new()
        .delete_requirement(*id)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn delete_test(
    _conn: &mut PgConnection,
    id: &i32,
) -> Result<bool, Box<dyn Error>> {
    DieselRepo::new()
        .delete_test(*id)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn delete_user(
    _conn: &mut PgConnection,
    id: &i32,
) -> Result<bool, Box<dyn Error>> {
    DieselRepo::new()
        .delete_user(*id)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn insert_new_test(
    _conn: &mut PgConnection,
    new: &NewTest,
) -> Result<i32, Box<dyn Error>> {
    DieselRepo::new()
        .insert_new_test(new)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn edit_test(
    _conn: &mut PgConnection,
    new: &NewTest,
) -> Result<bool, Box<dyn Error>> {
    DieselRepo::new()
        .edit_test(new)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn update_test_requirement_links(
    _conn: &mut PgConnection,
    test_id: i32,
    requirement_ids: &[i32],
) -> Result<(), Box<dyn Error>> {
    DieselRepo::new()
        .update_test_requirement_links(test_id, requirement_ids)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn insert_new_matrix_item(
    _conn: &mut PgConnection,
    new: &NewMatrix,
) -> Result<(), Box<dyn Error>> {
    DieselRepo::new()
        .insert_new_matrix_item(new)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn insert_new_user(
    _conn: &mut PgConnection,
    new: &NewUser,
) -> Result<i32, Box<dyn Error>> {
    DieselRepo::new()
        .insert_new_user(new)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn update_user(
    _conn: &mut PgConnection,
    user_data: &NewUser,
) -> Result<bool, Box<dyn Error>> {
    DieselRepo::new()
        .update_user(user_data)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn update_user_without_password(
    _conn: &mut PgConnection,
    user_data: &UpdateUser,
) -> Result<bool, Box<dyn Error>> {
    DieselRepo::new()
        .update_user_without_password(user_data)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn update_requirement(
    _conn: &mut PgConnection,
    req: i32,
) -> Result<(), Box<dyn Error>> {
    DieselRepo::new()
        .update_requirement(req)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn create_test(
    _conn: &mut PgConnection,
    new: &NewTest,
) -> Result<i32, Box<dyn Error>> {
    DieselRepo::new()
        .create_test(new)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn create_status(
    _conn: &mut PgConnection,
    new: &NewStatus,
) -> Result<i32, Box<dyn Error>> {
    DieselRepo::new()
        .create_status(new)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn create_user(
    _conn: &mut PgConnection,
    new: &NewUser,
) -> Result<i32, Box<dyn Error>> {
    DieselRepo::new()
        .create_user(new)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn insert_new_category(
    _conn: &mut PgConnection,
    new: &NewCategory,
) -> Result<i32, Box<dyn Error>> {
    DieselRepo::new()
        .insert_new_category(new)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn edit_category(
    _conn: &mut PgConnection,
    new: &NewCategory,
) -> Result<bool, Box<dyn Error>> {
    DieselRepo::new()
        .edit_category(new)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn delete_category(
    _conn: &mut PgConnection,
    id: &i32,
) -> Result<bool, Box<dyn Error>> {
    DieselRepo::new()
        .delete_category(*id)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn insert_new_project(
    _conn: &mut PgConnection,
    new: &NewProject,
) -> Result<i32, Box<dyn Error>> {
    DieselRepo::new()
        .insert_new_project(new)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn edit_project(
    _conn: &mut PgConnection,
    project_id_param: i32,
    update: &UpdateProject,
) -> Result<bool, Box<dyn Error>> {
    DieselRepo::new()
        .edit_project(project_id_param, update)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn delete_project(
    _conn: &mut PgConnection,
    project_id_param: &i32,
) -> Result<bool, Box<dyn Error>> {
    DieselRepo::new()
        .delete_project(*project_id_param)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn insert_new_applicability(
    _conn: &mut PgConnection,
    new: &NewApplicability,
) -> Result<i32, Box<dyn Error>> {
    DieselRepo::new()
        .insert_new_applicability(new)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn edit_applicability(
    _conn: &mut PgConnection,
    new: &NewApplicability,
) -> Result<bool, Box<dyn Error>> {
    DieselRepo::new()
        .edit_applicability(new)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

pub fn delete_applicability(
    _conn: &mut PgConnection,
    id: &i32,
) -> Result<bool, Box<dyn Error>> {
    DieselRepo::new()
        .delete_applicability(*id)
        .map_err(|e| Box::new(e) as Box<dyn Error>)
}

