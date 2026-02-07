erDiagram
    projects ||--o{ project_members : "has members"
    users ||--o{ project_members : "member of"
    projects ||--o{ requirement_status : "has"
    projects ||--o{ test_status : "has"
    projects ||--o{ categories : "has"
    projects ||--o{ applicability : "has"
    projects ||--o{ verification : "has"
    projects ||--o{ requirements : "contains"
    projects ||--o{ tests : "contains"
    projects ||--o{ matrix : "scoped by"
    projects ||--o{ logs : "project_id"
    projects ||--o{ requirement_embeddings : "scoped by"
    projects ||--o{ embedding_index_queue : "scoped by"

    requirements ||--o{ requirement_versions : "versions"
    requirements ||--o| requirement_versions : "current_version_id"
    requirement_versions }o--o| requirement_versions : "parent_id"
    requirement_versions }o--|| requirement_status : "status_id"
    requirement_versions }o--|| applicability : "applicability_id"
    requirement_versions ||--o{ requirement_version_verification_methods : "has"
    verification ||--o{ requirement_version_verification_methods : "used by"
    requirements ||--o{ requirement_embeddings : "embedding"
    requirements ||--o| embedding_index_queue : "queue entry"

    tests }o--|| test_status : "status_id"
    tests }o--o| tests : "parent_id"
    requirements ||--o{ matrix : "req_id"
    tests ||--o{ matrix : "test_id"

    users ||--o{ logs : "user_id"

    projects {
        serial id PK
        varchar name
        text description
        timestamp creation_date
        timestamp update_date
        varchar status
        int owner_id
    }

    users {
        serial id PK
        varchar username
        varchar name
        varchar email
        timestamp creation_date
        timestamp last_login
        varchar password_hash
        boolean is_admin
    }

    project_members {
        int project_id PK,FK
        int user_id PK,FK
        int role
        timestamp created_at
        timestamp updated_at
    }

    requirement_status {
        serial id PK
        varchar title
        varchar description
        varchar tag
        int project_id FK
    }

    test_status {
        serial id PK
        varchar title
        varchar description
        varchar tag
        int project_id FK
    }

    categories {
        serial id PK
        varchar title
        varchar description
        varchar tag
        int project_id FK
    }

    applicability {
        serial id PK
        varchar title
        varchar description
        varchar tag
        int project_id FK
    }

    verification {
        serial id PK
        varchar title
        varchar description
        varchar tag
        int project_id FK
    }

    requirements {
        serial id PK
        int project_id FK
        varchar stable_code
        int current_version_id FK
    }

    requirement_versions {
        serial id PK
        int requirement_id FK
        varchar title
        text description
        int status_id FK
        int author_id
        int reviewer_id
        int category_id
        int parent_id
        int applicability_id FK
        text justification
        timestamp deadline_date
        timestamp created_at
    }

    requirement_version_verification_methods {
        int requirement_version_id PK,FK
        int verification_method_id PK,FK
    }

    tests {
        serial id PK
        varchar name
        varchar reference_code
        text description
        varchar source
        int status_id FK
        int parent_id
        int project_id FK
    }

    matrix {
        int req_id PK,FK
        int test_id PK,FK
        timestamp creation_date
        int project_id FK
    }

    logs {
        serial log_id PK
        int user_id FK
        varchar action_type
        varchar entity_type
        int entity_id
        int project_id FK
        text old_values
        text new_values
        text description
        varchar ip_address
        text user_agent
        timestamp created_at
    }

    requirement_embeddings {
        int requirement_id PK,FK
        int project_id FK
        vector embedding
        varchar embedding_model
        varchar content_hash
        timestamp updated_at
    }

    embedding_index_queue {
        serial id PK
        int requirement_id FK
        int project_id FK
        varchar status
        text error_message
        timestamp created_at
        timestamp processed_at
    }