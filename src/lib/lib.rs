// use diesel::mysql::MysqlConnection;
use diesel::prelude::*;
use diesel::dsl::now;

use dotenvy::dotenv;
use std::env;
// use std::error::Error;

pub mod bbdd;
pub mod models;
pub mod schema;

use crate::models::*;

