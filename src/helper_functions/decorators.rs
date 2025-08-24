use crate::models::*;
use diesel::prelude::*;
use diesel::pg::PgConnection;
use std::error::Error;

use super::queries::{
    get_status_by_id,
    get_user_by_id,
    get_verification_by_id_safe,
    get_category_by_id_safe,
    get_applicability_by_id_safe,
    get_requirement_by_id_safe,
    get_test_by_id,
    get_status_name_by_id,
};

pub fn decorate_requirements(reqs: Vec<Requirement>) -> Vec<DecoratedRequirement> {
    let mut result = Vec::new();

    for r in reqs {
        let a = DecoratedRequirement {
            req_id: r.req_id,
            req_title: r.req_title,
            req_verification: get_verification_by_id_safe(r.req_verification, r.project_id).verification_name,
            req_description: r.req_description,
            req_current_status: get_status_by_id(r.req_current_status).st_title,
            req_current_status_id: r.req_current_status,
            req_author: if r.req_author != 0 {
                get_user_by_id(r.req_author).user_name
            } else {
                "".to_string()
            },
            req_reviewer: if r.req_reviewer != 0 {
                get_user_by_id(r.req_reviewer).user_name
            } else {
                "".to_string()
            },
            req_link: r.req_link,
            req_reference: r.req_reference,
            req_category: get_category_by_id_safe(r.req_category, r.project_id).cat_title,
            req_applicability: get_applicability_by_id_safe(r.req_applicability, r.project_id).app_title,
            req_parent_id: r.req_parent,
            req_parent_title: if r.req_parent != 0 {
                match get_requirement_by_id_safe(r.req_parent) {
                    Ok(parent_req) => parent_req.req_title,
                    Err(_) => "[Deleted Parent]".to_string()
                }
            } else {
                "".to_string()
            },
            req_creation_date: r.req_creation_date.format("%d-%m-%Y %H:%M:%S").to_string(),
            req_update_date: r.req_update_date.format("%d-%m-%Y %H:%M:%S").to_string(),
            req_deadline_date: r.req_deadline_date.format("%d-%m-%Y %H:%M:%S").to_string(),
            req_justification: r.req_justification,
            project_id: r.project_id,
        };
        result.push(a);
    }

    result
}

pub fn decorate_tests(tests: Vec<Test>) -> Vec<DecoratedTest> {
    let mut result = Vec::new();

    for r in tests {
        let a = DecoratedTest {
            test_id: r.test_id,
            test_name: r.test_name,
            test_description: r.test_description,
            test_source: r.test_source,
            test_status: get_status_name_by_id(r.test_status),
            test_status_id: r.test_status,
            test_parent_id: r.test_parent,
            test_parent_title: if r.test_parent != 0 {
                get_test_by_id(r.test_parent).test_name
            } else {
                "".to_string()
            },
            project_id: r.project_id,
        };
        #[cfg(debug_assertions)]
        println!("Decorate: {:?}", a);
        result.push(a);
    }

    result
}

pub fn get_linked_tests_for_requirement(conn: &mut PgConnection, req_id: i32) -> Result<Vec<DecoratedTest>, Box<dyn Error>> {
    use crate::schema::matrix::dsl::*;
    use crate::schema::tests::dsl::*;

    let linked_test_ids: Vec<i32> = matrix
        .filter(matrix_req_id.eq(req_id))
        .select(matrix_test_id)
        .load(conn)?;

    if linked_test_ids.is_empty() {
        return Ok(Vec::new());
    }

    let tests_data: Vec<Test> = tests
        .filter(test_id.eq_any(linked_test_ids))
        .load(conn)?;

    let mut decorated_tests = Vec::new();
    for test in tests_data {
        let status_name = get_status_name_by_id(test.test_status);
        let parent_title = if test.test_parent > 0 {
            get_test_by_id(test.test_parent).test_name
        } else {
            "None".to_string()
        };

        decorated_tests.push(DecoratedTest {
            test_id: test.test_id,
            test_name: test.test_name,
            test_description: test.test_description,
            test_source: test.test_source,
            test_status: status_name,
            test_status_id: test.test_status,
            test_parent_id: test.test_parent,
            test_parent_title: parent_title,
            project_id: test.project_id,
        });
    }

    Ok(decorated_tests)
}
