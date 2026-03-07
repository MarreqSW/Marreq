// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "tsvector", schema = "pg_catalog"))]
    pub struct Tsvector;
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    applicability (id) {
        id -> Int4,
        title -> Varchar,
        description -> Varchar,
        tag -> Varchar,
        project_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    baseline_requirements (baseline_id, requirement_id) {
        baseline_id -> Int4,
        requirement_id -> Int4,
        version_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    baseline_traceability (baseline_id, requirement_id, verification_id) {
        baseline_id -> Int4,
        requirement_id -> Int4,
        verification_id -> Int4,
        suspect -> Bool,
        suspect_at -> Nullable<Timestamp>,
        suspect_reason -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    baselines (id) {
        id -> Int4,
        project_id -> Int4,
        #[max_length = 255]
        name -> Varchar,
        description -> Nullable<Text>,
        created_at -> Timestamp,
        created_by -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    categories (id) {
        id -> Int4,
        title -> Varchar,
        description -> Varchar,
        tag -> Varchar,
        project_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    custom_field_definitions (id) {
        id -> Int4,
        project_id -> Int4,
        #[max_length = 255]
        label -> Varchar,
        #[max_length = 20]
        field_type -> Varchar,
        enum_values -> Nullable<Jsonb>,
        sort_order -> Int4,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    custom_field_values (requirement_version_id, custom_field_definition_id) {
        requirement_version_id -> Int4,
        custom_field_definition_id -> Int4,
        value -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    embedding_index_queue (id) {
        id -> Int4,
        requirement_id -> Int4,
        project_id -> Int4,
        #[max_length = 20]
        status -> Varchar,
        error_message -> Nullable<Text>,
        created_at -> Timestamp,
        processed_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

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
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    matrix (req_id, verification_id) {
        req_id -> Int4,
        verification_id -> Int4,
        creation_date -> Timestamp,
        project_id -> Int4,
        suspect -> Bool,
        suspect_at -> Nullable<Timestamp>,
        suspect_reason -> Nullable<Text>,
        cleared_by -> Nullable<Int4>,
        cleared_at -> Nullable<Timestamp>,
        triggering_version_id -> Nullable<Int4>,
        triggering_user_id -> Nullable<Int4>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    project_members (project_id, user_id) {
        project_id -> Int4,
        user_id -> Int4,
        role -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    projects (id) {
        id -> Int4,
        #[max_length = 255]
        name -> Varchar,
        description -> Nullable<Text>,
        creation_date -> Nullable<Timestamp>,
        update_date -> Nullable<Timestamp>,
        #[max_length = 50]
        status -> Varchar,
        owner_id -> Nullable<Int4>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    requirement_comments (id) {
        id -> Int4,
        requirement_id -> Int4,
        requirement_version_id -> Nullable<Int4>,
        author_id -> Int4,
        body -> Text,
        created_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    requirement_embeddings (requirement_id) {
        requirement_id -> Int4,
        project_id -> Int4,
        embedding -> Nullable<Vector>,
        #[max_length = 100]
        embedding_model -> Varchar,
        #[max_length = 64]
        content_hash -> Varchar,
        updated_at -> Timestamp,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    requirement_status (id) {
        id -> Int4,
        title -> Varchar,
        description -> Varchar,
        tag -> Varchar,
        project_id -> Int4,
        is_system -> Bool,
        #[max_length = 20]
        tag_color -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    requirement_version_links (id) {
        id -> Int4,
        source_version_id -> Int4,
        target_version_id -> Int4,
        #[max_length = 32]
        link_type -> Varchar,
        rationale -> Nullable<Text>,
        project_id -> Int4,
        created_at -> Timestamp,
        metadata -> Nullable<Jsonb>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    requirement_version_verification_methods (requirement_version_id, verification_method_id) {
        requirement_version_id -> Int4,
        verification_method_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;
    use super::sql_types::Tsvector;

    requirement_versions (id) {
        id -> Int4,
        requirement_id -> Int4,
        title -> Varchar,
        description -> Varchar,
        status_id -> Int4,
        author_id -> Int4,
        reviewer_id -> Int4,
        category_id -> Int4,
        applicability_id -> Int4,
        justification -> Nullable<Text>,
        deadline_date -> Nullable<Timestamp>,
        created_at -> Timestamp,
        search_vector -> Nullable<Tsvector>,
        #[max_length = 20]
        approval_state -> Varchar,
        approved_by -> Nullable<Int4>,
        approved_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    requirements (id) {
        id -> Int4,
        project_id -> Int4,
        stable_code -> Varchar,
        current_version_id -> Nullable<Int4>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    user_api_tokens (id) {
        id -> Int4,
        user_id -> Int4,
        #[max_length = 64]
        token_hash -> Varchar,
        #[max_length = 255]
        name -> Nullable<Varchar>,
        project_id -> Nullable<Int4>,
        created_at -> Timestamp,
        last_used_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

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
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    verification_methods (id) {
        id -> Int4,
        title -> Varchar,
        description -> Varchar,
        tag -> Varchar,
        project_id -> Int4,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    verification_status (id) {
        id -> Int4,
        title -> Varchar,
        description -> Varchar,
        tag -> Varchar,
        project_id -> Int4,
        is_system -> Bool,
        #[max_length = 20]
        tag_color -> Nullable<Varchar>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use pgvector::sql_types::*;

    verifications (id) {
        id -> Int4,
        name -> Varchar,
        reference_code -> Varchar,
        description -> Varchar,
        source -> Varchar,
        status_id -> Int4,
        parent_id -> Nullable<Int4>,
        project_id -> Int4,
        verification_method_id -> Nullable<Int4>,
    }
}

diesel::joinable!(applicability -> projects (project_id));
diesel::joinable!(baseline_requirements -> baselines (baseline_id));
diesel::joinable!(baseline_requirements -> requirement_versions (version_id));
diesel::joinable!(baseline_requirements -> requirements (requirement_id));
diesel::joinable!(baseline_traceability -> baselines (baseline_id));
diesel::joinable!(baseline_traceability -> requirements (requirement_id));
diesel::joinable!(baseline_traceability -> verifications (verification_id));
diesel::joinable!(baselines -> projects (project_id));
diesel::joinable!(baselines -> users (created_by));
diesel::joinable!(categories -> projects (project_id));
diesel::joinable!(custom_field_definitions -> projects (project_id));
diesel::joinable!(custom_field_values -> custom_field_definitions (custom_field_definition_id));
diesel::joinable!(custom_field_values -> requirement_versions (requirement_version_id));
diesel::joinable!(embedding_index_queue -> projects (project_id));
diesel::joinable!(embedding_index_queue -> requirements (requirement_id));
diesel::joinable!(logs -> projects (project_id));
diesel::joinable!(logs -> users (user_id));
diesel::joinable!(matrix -> projects (project_id));
diesel::joinable!(matrix -> requirement_versions (triggering_version_id));
diesel::joinable!(matrix -> requirements (req_id));
diesel::joinable!(matrix -> verifications (verification_id));
diesel::joinable!(project_members -> projects (project_id));
diesel::joinable!(project_members -> users (user_id));
diesel::joinable!(projects -> users (owner_id));
diesel::joinable!(requirement_comments -> requirement_versions (requirement_version_id));
diesel::joinable!(requirement_comments -> requirements (requirement_id));
diesel::joinable!(requirement_comments -> users (author_id));
diesel::joinable!(requirement_embeddings -> projects (project_id));
diesel::joinable!(requirement_embeddings -> requirements (requirement_id));
diesel::joinable!(requirement_status -> projects (project_id));
diesel::joinable!(requirement_version_links -> projects (project_id));
diesel::joinable!(requirement_version_verification_methods -> requirement_versions (requirement_version_id));
diesel::joinable!(requirement_version_verification_methods -> verification_methods (verification_method_id));
diesel::joinable!(requirement_versions -> applicability (applicability_id));
diesel::joinable!(requirement_versions -> categories (category_id));
diesel::joinable!(requirement_versions -> requirement_status (status_id));
diesel::joinable!(requirements -> projects (project_id));
diesel::joinable!(user_api_tokens -> projects (project_id));
diesel::joinable!(user_api_tokens -> users (user_id));
diesel::joinable!(verification_methods -> projects (project_id));
diesel::joinable!(verification_status -> projects (project_id));
diesel::joinable!(verifications -> projects (project_id));
diesel::joinable!(verifications -> verification_methods (verification_method_id));
diesel::joinable!(verifications -> verification_status (status_id));

diesel::allow_tables_to_appear_in_same_query!(
    applicability,
    baseline_requirements,
    baseline_traceability,
    baselines,
    categories,
    custom_field_definitions,
    custom_field_values,
    embedding_index_queue,
    logs,
    matrix,
    project_members,
    projects,
    requirement_comments,
    requirement_embeddings,
    requirement_status,
    requirement_version_links,
    requirement_version_verification_methods,
    requirement_versions,
    requirements,
    user_api_tokens,
    users,
    verification_methods,
    verification_status,
    verifications,
);
