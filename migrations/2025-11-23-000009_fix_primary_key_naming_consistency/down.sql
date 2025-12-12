-- Revert the column/sequence renames to their legacy equivalents.

-- Verification -----------------------------------------------------------------
ALTER TABLE verification
    DROP COLUMN IF EXISTS tag;

ALTER SEQUENCE IF EXISTS verification_id_seq
    OWNED BY NULL;

ALTER SEQUENCE IF EXISTS verification_id_seq
    RENAME TO verification_verification_id_seq;

ALTER TABLE verification
    RENAME COLUMN description TO verification_description,
    RENAME COLUMN title TO verification_name,
    RENAME COLUMN id TO verification_id;

-- Users ------------------------------------------------------------------------
ALTER SEQUENCE IF EXISTS users_id_seq
    OWNED BY NULL;

ALTER SEQUENCE IF EXISTS users_id_seq
    RENAME TO users_user_id_seq;

ALTER TABLE users
    RENAME COLUMN password_hash TO user_password,
    RENAME COLUMN last_login TO user_last_login,
    RENAME COLUMN creation_date TO user_creation_date,
    RENAME COLUMN email TO user_email,
    RENAME COLUMN name TO user_name,
    RENAME COLUMN username TO user_username,
    RENAME COLUMN id TO user_id;

-- Tests ------------------------------------------------------------------------
UPDATE tests
SET parent_id = COALESCE(parent_id, 0);

ALTER TABLE tests
    ALTER COLUMN parent_id SET NOT NULL;

ALTER SEQUENCE IF EXISTS tests_id_seq
    OWNED BY NULL;

ALTER SEQUENCE IF EXISTS tests_id_seq
    RENAME TO tests_test_id_seq;

ALTER TABLE tests
    RENAME COLUMN parent_id TO test_parent,
    RENAME COLUMN source TO test_source,
    RENAME COLUMN description TO test_description,
    RENAME COLUMN reference_code TO test_reference,
    RENAME COLUMN name TO test_name,
    RENAME COLUMN id TO test_id;

-- Requirements -----------------------------------------------------------------
UPDATE requirements
SET parent_id = COALESCE(parent_id, 0);

ALTER TABLE requirements
    ALTER COLUMN parent_id SET NOT NULL;

ALTER SEQUENCE IF EXISTS requirements_id_seq
    OWNED BY NULL;

ALTER SEQUENCE IF EXISTS requirements_id_seq
    RENAME TO requirements_req_id_seq;

ALTER TABLE requirements
    RENAME COLUMN justification TO req_justification,
    RENAME COLUMN applicability_id TO req_applicability,
    RENAME COLUMN deadline_date TO req_deadline_date,
    RENAME COLUMN update_date TO req_update_date,
    RENAME COLUMN creation_date TO req_creation_date,
    RENAME COLUMN parent_id TO req_parent,
    RENAME COLUMN category_id TO req_category,
    RENAME COLUMN reference_code TO req_reference,
    RENAME COLUMN reviewer_id TO req_reviewer,
    RENAME COLUMN author_id TO req_author,
    RENAME COLUMN status_id TO req_current_status,
    RENAME COLUMN verification_method_id TO req_verification,
    RENAME COLUMN description TO req_description,
    RENAME COLUMN title TO req_title,
    RENAME COLUMN id TO req_id;

-- Requirement status -----------------------------------------------------------
ALTER SEQUENCE IF EXISTS requirement_status_id_seq
    OWNED BY NULL;

ALTER SEQUENCE IF EXISTS requirement_status_id_seq
    RENAME TO requirement_status_req_st_id_seq;

ALTER TABLE requirement_status
    RENAME COLUMN tag TO req_st_short_name,
    RENAME COLUMN description TO req_st_description,
    RENAME COLUMN title TO req_st_title,
    RENAME COLUMN id TO req_st_id;

-- Projects ---------------------------------------------------------------------
ALTER SEQUENCE IF EXISTS projects_id_seq
    OWNED BY NULL;

ALTER SEQUENCE IF EXISTS projects_id_seq
    RENAME TO projects_project_id_seq;

ALTER TABLE projects
    RENAME COLUMN owner_id TO project_owner_id,
    RENAME COLUMN update_date TO project_update_date,
    RENAME COLUMN creation_date TO project_creation_date,
    RENAME COLUMN description TO project_description,
    RENAME COLUMN name TO project_name,
    RENAME COLUMN id TO project_id;

-- Matrix -----------------------------------------------------------------------
ALTER TABLE matrix
    RENAME COLUMN creation_date TO matrix_creation_date,
    RENAME COLUMN test_id TO matrix_test_id,
    RENAME COLUMN req_id TO matrix_req_id;

-- Categories -------------------------------------------------------------------
ALTER SEQUENCE IF EXISTS categories_id_seq
    OWNED BY NULL;

ALTER SEQUENCE IF EXISTS categories_id_seq
    RENAME TO categories_cat_id_seq;

ALTER TABLE categories
    RENAME COLUMN tag TO cat_tag,
    RENAME COLUMN description TO cat_description,
    RENAME COLUMN title TO cat_title,
    RENAME COLUMN id TO cat_id;

-- Applicability ----------------------------------------------------------------
ALTER SEQUENCE IF EXISTS applicability_id_seq
    OWNED BY NULL;

ALTER SEQUENCE IF EXISTS applicability_id_seq
    RENAME TO applicability_app_id_seq;

ALTER TABLE applicability
    RENAME COLUMN tag TO app_tag,
    RENAME COLUMN description TO app_description,
    RENAME COLUMN title TO app_title,
    RENAME COLUMN id TO app_id;
