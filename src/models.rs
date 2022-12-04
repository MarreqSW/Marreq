use diesel::prelude::*;
use crate::schema::*;
use std::fmt;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Queryable)]
pub struct Requirement {
    pub req_id: i32,
    pub req_title: String,
    pub req_description: String,
    pub req_status: i32,
    pub req_author: String,
    pub req_author_email: String,
    pub req_link: String,
    pub req_reference: String,
    pub req_category: i32,
    pub req_creation_date: chrono::NaiveDateTime,
    pub req_update_date: chrono::NaiveDateTime,
    pub req_deadline_date: chrono::NaiveDateTime,
}

#[derive(Serialize, Deserialize, Insertable)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = requirements)]
pub struct NewRequirement {
    pub req_title: String,
    pub req_description: String,
    pub req_author: String,
    pub req_author_email: String,
    pub req_link: String,
    pub req_category: i32,
    pub req_current_status: i32,   
}

#[derive(Serialize, Deserialize, Queryable)]
pub struct Category {
    pub cat_id: i32,
    pub cat_title: String,
    pub cat_description: String,
    pub cat_tag: String,
}

#[derive(Serialize, Deserialize, Queryable)]
pub struct Status {
    pub st_id: i32,
    pub st_title: String,
    pub st_description: String,
    pub st_short_name: String,
}

#[derive(Serialize, Deserialize, Queryable)]
pub struct Matrix {
    pub matrix_req_id: i32,
    pub matrix_test_id: i32,
    pub matrix_creation_date: chrono::NaiveDateTime
}

#[derive(Serialize, Deserialize, Queryable)]
pub struct Tests {
    pub test_id: i32,
    pub test_name: String,
    pub test_description: String,
    pub test_source: i32,
    pub test_status: i32
}

impl fmt::Display for Requirement {
    fn fmt (&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<p id='requirement'>
        <p id='RegNum'>Num: {}</p>
        <p id='ReqTitle'>Title: {}</p><p id='ReqDesc'>Description: {}</p>
        <p id='ReqAuthor'>Author: {}</p><p id='ReqRef'>Reference {}</p>
        <p id='ReqDate'>Date: {}</p>", 
        self.req_id, self.req_title, self.req_description, self.req_author, self.req_reference, self.req_creation_date)
    }
}


impl fmt::Display for NewRequirement {
    fn fmt (&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<p id='requirement'>
        <p id='ReqTitle'>Title: {}</p><p id='ReqDesc'>Description: {}</p>
        <p id='ReqAuthor'>Author: {}</p>",
        self.req_title, self.req_description, self.req_author)
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<p id='category'>Category: {}</p>", self.cat_title)       
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<p id='status'>Status: {}</p>", self.st_title)
    }       
}

impl fmt::Display for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "
        <p id='matrixID'>Req ID: {}</p>
        <p id='matrixID'>Test ID: {}</p>", 
        self.matrix_req_id, self.matrix_test_id)
    }
}

impl fmt::Display for Tests {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "
        <p id='testID'>{}</p>
        <p id='testName'>{}</p>
        <p id='testDescription'>{}</p>
        <p id='testSource'>{}</p>
        ", self.test_id, self.test_name, self.test_description, self.test_source)
    }
}