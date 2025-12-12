-- Rename legacy columns (and their sequences) to the new Diesel naming convention.

-- Applicability ----------------------------------------------------------------
ALTER TABLE applicability
    RENAME COLUMN app_id TO id,
    RENAME COLUMN app_title TO title,
    RENAME COLUMN app_description TO description,
    RENAME COLUMN app_tag TO tag;

ALTER SEQUENCE IF EXISTS applicability_app_id_seq
    RENAME TO applicability_id_seq;

ALTER SEQUENCE IF EXISTS applicability_id_seq
    OWNED BY applicability.id;

-- Categories -------------------------------------------------------------------
ALTER TABLE categories
    RENAME COLUMN cat_id TO id,
    RENAME COLUMN cat_title TO title,
    RENAME COLUMN cat_description TO description,
    RENAME COLUMN cat_tag TO tag;

ALTER SEQUENCE IF EXISTS categories_cat_id_seq
    RENAME TO categories_id_seq;

ALTER SEQUENCE IF EXISTS categories_id_seq
    OWNED BY categories.id;

-- Matrix -----------------------------------------------------------------------
ALTER TABLE matrix
    RENAME COLUMN matrix_req_id TO req_id,
    RENAME COLUMN matrix_test_id TO test_id,
    RENAME COLUMN matrix_creation_date TO creation_date;

-- Projects ---------------------------------------------------------------------
ALTER TABLE projects
    RENAME COLUMN project_id TO id,
    RENAME COLUMN project_name TO name,
    RENAME COLUMN project_description TO description,
    RENAME COLUMN project_creation_date TO creation_date,
    RENAME COLUMN project_update_date TO update_date,
    RENAME COLUMN project_owner_id TO owner_id;

ALTER SEQUENCE IF EXISTS projects_project_id_seq
    RENAME TO projects_id_seq;

ALTER SEQUENCE IF EXISTS projects_id_seq
    OWNED BY projects.id;

-- Requirement status -----------------------------------------------------------
ALTER TABLE requirement_status
    RENAME COLUMN req_st_id TO id,
    RENAME COLUMN req_st_title TO title,
    RENAME COLUMN req_st_description TO description,
    RENAME COLUMN req_st_short_name TO tag;

ALTER SEQUENCE IF EXISTS requirement_status_req_st_id_seq
    RENAME TO requirement_status_id_seq;

ALTER SEQUENCE IF EXISTS requirement_status_id_seq
    OWNED BY requirement_status.id;

-- Requirements -----------------------------------------------------------------
ALTER TABLE requirements
    RENAME COLUMN req_id TO id,
    RENAME COLUMN req_title TO title,
    RENAME COLUMN req_description TO description,
    RENAME COLUMN req_verification TO verification_method_id,
    RENAME COLUMN req_current_status TO status_id,
    RENAME COLUMN req_author TO author_id,
    RENAME COLUMN req_reviewer TO reviewer_id,
    RENAME COLUMN req_reference TO reference_code,
    RENAME COLUMN req_category TO category_id,
    RENAME COLUMN req_parent TO parent_id,
    RENAME COLUMN req_creation_date TO creation_date,
    RENAME COLUMN req_update_date TO update_date,
    RENAME COLUMN req_deadline_date TO deadline_date,
    RENAME COLUMN req_applicability TO applicability_id,
    RENAME COLUMN req_justification TO justification;

ALTER SEQUENCE IF EXISTS requirements_req_id_seq
    RENAME TO requirements_id_seq;

ALTER SEQUENCE IF EXISTS requirements_id_seq
    OWNED BY requirements.id;

ALTER TABLE requirements
    ALTER COLUMN parent_id DROP NOT NULL;

-- Tests ------------------------------------------------------------------------
ALTER TABLE tests
    RENAME COLUMN test_id TO id,
    RENAME COLUMN test_name TO name,
    RENAME COLUMN test_reference TO reference_code,
    RENAME COLUMN test_description TO description,
    RENAME COLUMN test_source TO source,
    RENAME COLUMN test_parent TO parent_id;

ALTER SEQUENCE IF EXISTS tests_test_id_seq
    RENAME TO tests_id_seq;

ALTER SEQUENCE IF EXISTS tests_id_seq
    OWNED BY tests.id;

ALTER TABLE tests
    ALTER COLUMN parent_id DROP NOT NULL;

-- Users ------------------------------------------------------------------------
ALTER TABLE users
    RENAME COLUMN user_id TO id,
    RENAME COLUMN user_username TO username,
    RENAME COLUMN user_name TO name,
    RENAME COLUMN user_email TO email,
    RENAME COLUMN user_creation_date TO creation_date,
    RENAME COLUMN user_last_login TO last_login,
    RENAME COLUMN user_password TO password_hash;

ALTER SEQUENCE IF EXISTS users_user_id_seq
    RENAME TO users_id_seq;

ALTER SEQUENCE IF EXISTS users_id_seq
    OWNED BY users.id;

-- Verification -----------------------------------------------------------------
ALTER TABLE verification
    RENAME COLUMN verification_id TO id,
    RENAME COLUMN verification_name TO title,
    RENAME COLUMN verification_description TO description;

ALTER SEQUENCE IF EXISTS verification_verification_id_seq
    RENAME TO verification_id_seq;

ALTER SEQUENCE IF EXISTS verification_id_seq
    OWNED BY verification.id;

ALTER TABLE verification
    ADD COLUMN tag VARCHAR NOT NULL DEFAULT 'GEN';

ALTER TABLE verification
    ALTER COLUMN tag DROP DEFAULT;
