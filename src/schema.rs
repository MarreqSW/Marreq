// @generated automatically by Diesel CLI.

diesel::table! {
    categories (cat_id) {
        cat_id -> Int4,
        cat_title -> Varchar,
        cat_description -> Varchar,
        cat_tag -> Varchar,
    }
}

diesel::table! {
    matrix (matrix_req_id, matrix_test_id) {
        matrix_req_id -> Int4,
        matrix_test_id -> Int4,
        matrix_creation_date -> Timestamp,
    }
}

diesel::table! {
    requirements (req_id) {
        req_id -> Int4,
        req_title -> Varchar,
        req_description -> Varchar,
        req_current_status -> Int4,
        req_author -> Varchar,
        req_author_email -> Varchar,
        req_link -> Varchar,
        req_reference -> Varchar,
        req_category -> Int4,
        req_parent -> Int4,
        req_creation_date -> Timestamp,
        req_update_date -> Timestamp,
        req_deadline_date -> Timestamp,
    }
}

diesel::table! {
    status (st_id) {
        st_id -> Int4,
        st_title -> Varchar,
        st_description -> Varchar,
        st_short_name -> Varchar,
    }
}

diesel::table! {
    tests (test_id) {
        test_id -> Int4,
        test_name -> Varchar,
        test_description -> Varchar,
        test_source -> Varchar,
        test_status -> Int4,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    categories,
    matrix,
    requirements,
    status,
    tests,
);
