// @generated automatically by Diesel CLI.

diesel::table! {
    applicability (app_id) {
        app_id -> Int4,
        app_title -> Varchar,
        app_description -> Varchar,
        app_tag -> Varchar,
    }
}

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
        req_verification -> Int4,
        req_current_status -> Int4,
        req_author -> Int4,
        req_reviewer -> Int4,
        req_link -> Varchar,
        req_reference -> Varchar,
        req_category -> Int4,
        req_parent -> Int4,
        req_creation_date -> Timestamp,
        req_update_date -> Timestamp,
        req_deadline_date -> Timestamp,
        req_applicability -> Int4,
        req_justification -> Nullable<Text>,
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
        test_parent -> Int4,
    }
}

diesel::table! {
    users (user_id) {
        user_id -> Int4,
        user_username -> Varchar,
        user_name -> Varchar,
        user_email -> Varchar,
        user_level -> Int4,
        user_creation_date -> Timestamp,
        user_last_login -> Timestamp,
    }
}

diesel::table! {
    verification (verification_id) {
        verification_id -> Int4,
        verification_name -> Varchar,
        verification_description -> Varchar,
    }
}

diesel::joinable!(requirements -> applicability (req_applicability));

diesel::allow_tables_to_appear_in_same_query!(
    applicability,
    categories,
    matrix,
    requirements,
    status,
    tests,
    users,
    verification,
);
