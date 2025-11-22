// @generated automatically by Diesel CLI.

diesel::table! {
    applicability (id) {
        id -> Int4,
        title -> Varchar,
        description -> Varchar,
        tag -> Varchar,
        project_id -> Int4,
    }
}

diesel::table! {
    categories (id) {
        id -> Int4,
        title -> Varchar,
        description -> Varchar,
        tag -> Varchar,
        project_id -> Int4,
    }
}

diesel::table! {
    logs (log_id) {
        log_id -> Int4,
        id -> Int4,
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
    matrix (req_id, test_id) {
        req_id -> Int4,
        test_id -> Int4,
        creation_date -> Timestamp,
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
    projects (id) {
        id -> Int4,
        #[max_length = 255]
        name -> Varchar,
        description -> Nullable<Text>,
        creation_date -> Nullable<Timestamp>,
        update_date -> Nullable<Timestamp>,
        #[max_length = 50]
        status_id -> Nullable<Varchar>,
        owner_id -> Nullable<Int4>,
    }
}

diesel::table! {
    requirement_status (id) {
        id -> Int4,
        title -> Varchar,
        description -> Varchar,
        tag -> Varchar,
        project_id -> Int4,
    }
}

diesel::table! {
    requirements (id) {
        id -> Int4,
        title -> Varchar,
        description -> Varchar,
        verification_method_id -> Int4,
        status_id -> Int4,
        author_id -> Int4,
        reviewer_id -> Int4,
        reference_code -> Varchar,
        category_id -> Int4,
        parent_id -> Nullable<Int4>,
        creation_date -> Timestamp,
        update_date -> Timestamp,
        deadline_date -> Timestamp,
        applicability_id -> Int4,
        justification -> Nullable<Text>,
        project_id -> Int4,
    }
}

diesel::table! {
    status_id (id) {
        id -> Int4,
        title -> Varchar,
        description -> Varchar,
        tag -> Varchar,
        project_id -> Int4,
    }
}

diesel::table! {
    tests (id) {
        id -> Int4,
        name -> Varchar,
        reference_code -> Varchar,
        description -> Varchar,
        source -> Varchar,
        status_id -> Int4,
        parent_id -> Nullable<Int4>,
        project_id -> Int4,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        username -> Varchar,
        name -> Varchar,
        email -> Varchar,
        creation_date -> Timestamp,
        last_login -> Timestamp,
        #[max_length = 255]
        password_hash -> Varchar,
        is_admin -> Bool,
    }
}

diesel::table! {
    verification (id) {
        id -> Int4,
        title -> Varchar,
        description -> Varchar,
        tag -> Varchar,
        project_id -> Int4,
    }
}

diesel::joinable!(applicability -> projects (project_id));
diesel::joinable!(categories -> projects (project_id));
diesel::joinable!(logs -> projects (project_id));
diesel::joinable!(logs -> users (id));
diesel::joinable!(matrix -> projects (project_id));
diesel::joinable!(matrix -> requirements (req_id));
diesel::joinable!(matrix -> tests (test_id));
diesel::joinable!(project_members -> projects (project_id));
diesel::joinable!(project_members -> users (user_id));
diesel::joinable!(requirement_status -> projects (project_id));
diesel::joinable!(requirements -> applicability (applicability_id));
diesel::joinable!(requirements -> projects (project_id));
diesel::joinable!(requirements -> requirement_status (status_id));
diesel::joinable!(status_id -> projects (project_id));
diesel::joinable!(tests -> projects (project_id));
diesel::joinable!(tests -> status_id (status_id));
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
    status_id,
    tests,
    users,
    verification,
);
