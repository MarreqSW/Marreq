use crate::schema::*;
use diesel::prelude::*;
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Queryable, AsChangeset)]
pub struct Requirement {
    pub req_id: i32,
    pub req_title: String,
    pub req_description: String,
    pub req_verification: i32,
    pub req_current_status: i32,
    pub req_author: i32,
    pub req_reviewer: i32,
    pub req_link: String,
    pub req_reference: String,
    pub req_category: i32,
    pub req_parent: i32,
    pub req_creation_date: chrono::NaiveDateTime,
    pub req_update_date: chrono::NaiveDateTime,
    pub req_deadline_date: chrono::NaiveDateTime,
}

#[derive(Serialize, Deserialize, Insertable, AsChangeset, FromForm)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = requirements)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(req_id))]
pub struct NewRequirement {
    pub req_id: Option<i32>,
    pub req_title: String,
    pub req_description: String,
    pub req_verification: i32,
    pub req_author: i32,
    pub req_link: String,
    pub req_category: i32,
    pub req_current_status: i32,
    pub req_parent: i32,
    pub req_reference: String,
    pub req_reviewer: i32,
}

#[derive(Serialize, Deserialize)]
pub struct DecoratedRequirement {
    pub req_id: i32,
    pub req_title: String,
    pub req_description: String,
    pub req_verification: String,
    pub req_current_status: String,
    pub req_author: String,
    pub req_reviewer: String,
    pub req_link: String,
    pub req_reference: String,
    pub req_category: String,
    pub req_parent_id: i32,
    pub req_parent_title: String,
    pub req_creation_date: String,
    pub req_update_date: String,
    pub req_deadline_date: String,
}

#[derive(Serialize, Deserialize, Queryable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Category {
    pub cat_id: i32,
    pub cat_title: String,
    pub cat_description: String,
    pub cat_tag: String,
}

#[derive(Serialize, Deserialize, Queryable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Status {
    pub st_id: i32,
    pub st_title: String,
    pub st_description: String,
    pub st_short_name: String,
}

#[derive(Serialize, Deserialize, Queryable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Verification {
    pub ver_id: i32,
    pub ver_title: String,
    pub ver_description: String,
}

#[derive(Serialize, Deserialize, Queryable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Matrix {
    pub matrix_req_id: i32,
    pub matrix_test_id: i32,
    pub matrix_creation_date: chrono::NaiveDateTime,
}

#[derive(Serialize, Deserialize, Insertable)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = matrix)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewMatrix {
    pub matrix_req_id: i32,
    pub matrix_test_id: i32,
}

#[derive(Serialize, Deserialize, Queryable, AsChangeset, Debug)]
pub struct User {
    pub user_id: i32,
    pub user_username: String,
    pub user_name: String,
    pub user_email: String,
    pub user_level: i32,
    pub user_creation_date: chrono::NaiveDateTime,
    pub user_last_login: chrono::NaiveDateTime,
}

#[derive(Serialize, Deserialize, Queryable, Insertable, AsChangeset, FromForm)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[diesel(primary_key(user_id))]
pub struct NewUser {
    pub user_id: Option<i32>,
    pub user_username: String,
    pub user_name: String,
    pub user_email: String,
    pub user_level: i32,
}

#[derive(Serialize, Deserialize, Queryable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Test {
    pub test_id: i32,
    pub test_name: String,
    pub test_description: String,
    pub test_source: String,
    pub test_status: i32,
    pub test_parent: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DecoratedTest {
    pub test_id: i32,
    pub test_name: String,
    pub test_description: String,
    pub test_source: String,
    pub test_status: String,
    pub test_parent_id: i32,
    pub test_parent_title: String,
}

#[derive(Serialize, Deserialize, Insertable, FromForm, AsChangeset)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = tests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewTest {
    pub test_id: Option<i32>,
    pub test_name: String,
    pub test_description: String,
    pub test_source: String,
    pub test_status: i32,
    pub test_parent: i32,
}

#[derive(Serialize, Deserialize, FromForm)]
#[serde(crate = "rocket::serde")]
pub struct NewTestForm {
    pub test_name: String,
    pub test_description: String,
    pub test_source: String,
    pub test_status: i32,
    pub test_parent: i32,
    pub test_req: Vec<i32>,
}

impl fmt::Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "
        <div class='requirement'>
            <div class='ReqNum'>Num: <a href='http://localhost:8000/requirements/{}'>{}</a></div>
            <div class='ReqTitle'>Title: {}</div>
            <div class='ReqDesc'>Description: {}</div>
            <div class='ReqAuthor'>Author: {}</div>
            <div class='ReqRef'>Reference {}</div>
            <div class='ReqDate'>Date: {}</div>
            <div class='ReqParent'>Parent: {}</div>
        </div>",
            self.req_id,
            self.req_id,
            self.req_title,
            self.req_description,
            self.req_author,
            self.req_reference,
            self.req_creation_date,
            self.req_parent
        )
    }
}

impl fmt::Display for NewRequirement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "
        <div class='requirement'>
            <div class='ReqTitle'>Title: {}</div><div class='ReqDesc'>Description: {}</div>
            <div class='ReqAuthor'>Author: {}</div>
        </div>",
            self.req_title, self.req_description, self.req_author
        )
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "<div class='category'>Category: {}</div>",
            self.cat_title
        )
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<div class='status'>Status: {}</div>", self.st_title)
    }
}

impl fmt::Display for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "
        <div class='matrixID'>Req ID: {}</div>
        <div class='matrixID'>Test ID: {}</div>",
            self.matrix_req_id, self.matrix_test_id
        )
    }
}

impl fmt::Display for Test {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "
        <div class='TestDiv'>
        <div class='testID'>Test ID: <a href='http://localhost:8000/tests/{}'>{}</a></div>
        <div class='testName'>Name: {}</div>
        <div class='testDescription'>Description: {}</div>
        <div class='testSource'>Source: {}</div>
        <div class='testParent'>Parent: {}</div>
        </div>
        ",
            self.test_id,
            self.test_id,
            self.test_name,
            self.test_description,
            self.test_source,
            self.test_parent
        )
    }
}
