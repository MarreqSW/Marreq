// @generated automatically by Diesel CLI.

diesel::table! {
    applicability (app_id) {
        app_id -> Int4,
        app_title -> Varchar,
        app_description -> Varchar,
        app_tag -> Varchar,
        project_id -> Int4,
    }
}

diesel::table! {
    categories (cat_id) {
        cat_id -> Int4,
        cat_title -> Varchar,
        cat_description -> Varchar,
        cat_tag -> Varchar,
        project_id -> Int4,
    }
}

diesel::table! {
    logs (log_id) {
        log_id -> Int4,
        user_id -> Int4,
        #[max_length = 50]
        action_type -> Varchar,
        #[max_length = 50]
        entity_type -> Varchar,
        entity_id -> Nullable<Int4>,
        project_id -> Nullable<Int4>,
        old_values -> Nullable<Text>,
        new_values -> Nullable<Text>,
        description -> Nullable<Text>,
        #[max_length = 45]
        ip_address -> Nullable<Varchar>,
        user_agent -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    matrix (matrix_req_id, matrix_test_id) {
        matrix_req_id -> Int4,
        matrix_test_id -> Int4,
        matrix_creation_date -> Timestamp,
        project_id -> Int4,
    }
}

diesel::table! {
    project_members (project_id, user_id) {
        project_id -> Int4,
        user_id -> Int4,
        role -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    projects (project_id) {
        project_id -> Int4,
        #[max_length = 255]
        project_name -> Varchar,
        project_description -> Nullable<Text>,
        project_creation_date -> Nullable<Timestamp>,
        project_update_date -> Nullable<Timestamp>,
        #[max_length = 50]
        project_status -> Nullable<Varchar>,
        project_owner_id -> Nullable<Int4>,
    }
}

diesel::table! {
    requirement_status (req_st_id) {
        req_st_id -> Int4,
        req_st_title -> Varchar,
        req_st_description -> Varchar,
        req_st_short_name -> Varchar,
    }
}

diesel::table! {
    requirements (req_id) {
        req_id -> Int4,
        req_title -> Varchar,
        req_description -> Varchar,
        req_verification_method -> Int4,
        req_current_status -> Int4,
        req_author -> Int4,
        req_reviewer -> Int4,
        req_reference -> Varchar,
        req_category -> Int4,
        req_parent -> Int4,
        req_creation_date -> Timestamp,
        req_update_date -> Timestamp,
        req_deadline_date -> Timestamp,
        req_applicability -> Int4,
        req_justification -> Nullable<Text>,
        project_id -> Int4,
    }
}

diesel::table! {
    test_status (test_st_id) {
        test_st_id -> Int4,
        test_st_title -> Varchar,
        test_st_description -> Varchar,
        test_st_short_name -> Varchar,
    }
}

diesel::table! {
    tests (test_id) {
        test_id -> Int4,
        test_name -> Varchar,
        test_reference -> Varchar,
        test_description -> Varchar,
        test_source -> Varchar,
        test_status -> Int4,
        test_parent -> Int4,
        project_id -> Int4,
    }
}

diesel::table! {
    users (user_id) {
        user_id -> Int4,
        user_username -> Varchar,
        user_name -> Varchar,
        user_email -> Varchar,
        user_creation_date -> Timestamp,
        user_last_login -> Timestamp,
        #[max_length = 255]
        user_password -> Varchar,
        is_admin -> Bool,
    }
}

diesel::table! {
    verification (verification_id) {
        verification_id -> Int4,
        verification_name -> Varchar,
        verification_description -> Varchar,
        project_id -> Int4,
    }
}

diesel::joinable!(applicability -> projects (project_id));
diesel::joinable!(categories -> projects (project_id));
diesel::joinable!(logs -> projects (project_id));
diesel::joinable!(logs -> users (user_id));
diesel::joinable!(matrix -> projects (project_id));
diesel::joinable!(matrix -> requirements (matrix_req_id));
diesel::joinable!(matrix -> tests (matrix_test_id));
diesel::joinable!(project_members -> projects (project_id));
diesel::joinable!(project_members -> users (user_id));
diesel::joinable!(requirements -> applicability (req_applicability));
diesel::joinable!(requirements -> projects (project_id));
diesel::joinable!(requirements -> requirement_status (req_current_status));
diesel::joinable!(tests -> projects (project_id));
diesel::joinable!(tests -> test_status (test_status));
diesel::joinable!(verification -> projects (project_id));

diesel::allow_tables_to_appear_in_same_query!(
    applicability,
    categories,
    logs,
    matrix,
    project_members,
    projects,
    requirement_status,
    requirements,
    test_status,
    tests,
    users,
    verification,
);
