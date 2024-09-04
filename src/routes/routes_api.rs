use diesel::prelude::*;
use rocket::serde::json::{json, Json, Value};

use crate::helper_functions::*;
use crate::models::*;

// --------------------------------
// API Routes
// --------------------------------

/// Requirements
#[get("/requirements")]
pub fn api_get_requirement() -> Result<Json<Vec<Requirement>>, rocket::http::Status> {
    let ret_val = get_requirements_all()
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    if let Ok(val) = ret_val {
        Ok(val)
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}

#[post("/requirements", data = "<new_req>")]
pub async fn api_post_requirement(
    new_req: Json<NewRequirement>,
) -> Result<Value, rocket::http::Status> {
    let connection = &mut establish_connection();
    let ret_value = insert_new_requirement(connection, &new_req);

    if let Ok(val) = ret_value {
        Ok(json!({ "status": "ok", "id": val }))
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[delete("/requirements/<ident>")]
pub async fn api_delete_requirement_by_id(ident: i32) -> rocket::http::Status {
    let connection = &mut establish_connection();
    let ret_value = delete_requirement(connection, &ident).unwrap();

    println!("Delete value: {}", ret_value);
    if ret_value {
        rocket::http::Status::NoContent
    } else {
        rocket::http::Status::Accepted
    }
}

#[get("/requirements/<ident>")]
pub fn api_get_requirement_by_id(
    ident: i32,
) -> Result<Json<Vec<Requirement>>, rocket::http::Status> {
    use crate::schema::requirements::dsl::*;
    let connection = &mut establish_connection();

    let ret_val = requirements
        .filter(req_id.eq(ident))
        .load::<Requirement>(connection)
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    if let Ok(val) = ret_val {
        if val.is_empty() {
            Err(rocket::http::Status::NotFound)
        } else {
            Ok(val)
        }
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}

/// Categories
#[get("/categories")]
pub fn api_get_categories() -> Result<Json<Vec<Category>>, rocket::http::Status> {
    let ret_val = get_categories_all()
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    if let Ok(val) = ret_val {
        if val.is_empty() {
            Err(rocket::http::Status::NotFound)
        } else {
            Ok(val)
        }
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}

/// Status
#[get("/status")]
pub fn api_get_status() -> Result<Json<Vec<Status>>, rocket::http::Status> {
    let ret_val = get_status_all()
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .map(Json);
    if let Ok(val) = ret_val {
        if val.is_empty() {
            Err(rocket::http::Status::NotFound)
        } else {
            Ok(val)
        }
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}

#[post("/status", data= "<new_status>")]
pub async fn api_post_status(new_status: Json<NewStatus>) -> Value {
    let connection = &mut establish_connection();
    let new_id = create_status (connection, &new_status).unwrap();

    json!({ "status": "ok", "id": new_id })
}

/// Tests
#[get("/tests")]
pub fn api_get_test() -> Result<Json<Vec<Test>>, rocket::http::Status> {
    let ret_val = get_tests_all()
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    if let Ok(val) = ret_val {
        Ok(val)
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}

#[get("/tests/<ident>")]
pub fn api_get_test_by_id(ident: i32) -> Result<Json<Vec<Test>>, rocket::http::Status> {
    use crate::schema::tests::dsl::*;
    let connection = &mut establish_connection();

    let ret_val = tests
        .filter(test_id.eq(ident))
        .load::<Test>(connection)
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    if let Ok(val) = ret_val {
        if val.is_empty() {
            Err(rocket::http::Status::NotFound)
        } else {
            Ok(val)
        }
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}

#[post("/tests", data = "<new_test>")]
pub async fn api_post_test(new_test: Json<NewTest>) -> Result<Value, rocket::http::Status> {
    let connection = &mut establish_connection();
    let ret_value = create_test(connection, &new_test);

    if let Ok(val) = ret_value {
        Ok(json!({ "status": "ok", "id": val }))
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[delete("/tests/<ident>")]
pub async fn api_delete_test_by_id(ident: i32) -> rocket::http::Status {
    let connection = &mut establish_connection();
    let ret_value = delete_test(connection, &ident).unwrap();

    if ret_value {
        rocket::http::Status::NoContent
    } else {
        rocket::http::Status::Accepted
    }
}

/// Users
#[get("/users")]
pub fn api_get_users() -> Result<Json<Vec<User>>, rocket::http::Status> {
    let ret_val = get_users_all()
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    if let Ok(val) = ret_val {
        Ok(val)
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}

#[get("/users/<ident>")]
pub fn api_get_users_by_id(ident: i32) -> Result<Json<Vec<User>>, rocket::http::Status> {
    use crate::schema::users::dsl::*;
    let connection = &mut establish_connection();

    let ret_val = users
        .filter(user_id.eq(ident))
        .load::<User>(connection)
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    if let Ok(val) = ret_val {
        if val.is_empty() {
            Err(rocket::http::Status::NotFound)
        } else {
            Ok(val)
        }
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}

#[post("/users", data = "<new_user>")]
pub async fn api_post_user(new_user: Json<NewUser>) -> Result<Value, rocket::http::Status> {
    let connection = &mut establish_connection();
    let ret_value = create_user(connection, &new_user);

    if let Ok(val) = ret_value {
        Ok(json!({ "status": "ok", "id": val }))
    } else {
        Err(rocket::http::Status::BadRequest)
    }
}

#[delete("/users/<ident>")]
pub async fn api_delete_user_by_id(ident: i32) -> rocket::http::Status {
    let connection = &mut establish_connection();
    let ret_value = delete_user(connection, &ident);

    if let Ok(val) = ret_value {
        if val {
            rocket::http::Status::NoContent
        } else {
            rocket::http::Status::Accepted
        }
    } else {
        rocket::http::Status::BadRequest
    }
}

/// Matrix

#[get("/matrix")]
pub fn api_get_matrix() -> Result<Json<Vec<Matrix>>, rocket::http::Status> {
    use crate::schema::matrix::dsl::*;
    let connection = &mut establish_connection();

    let ret_val = matrix
        .load::<Matrix>(connection)
        .map_err(|err| -> String {
            println!("Error querying page views: {:?}", err);
            "Error querying page views from the database".into()
        })
        .map(Json);

    if let Ok(val) = ret_val {
        if val.is_empty() {
            Err(rocket::http::Status::NotFound)
        } else {
            Ok(val)
        }
    } else {
        Err(rocket::http::Status::InternalServerError)
    }
}
