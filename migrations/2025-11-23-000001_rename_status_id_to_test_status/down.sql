-- Revert the test status normalization.

ALTER TABLE test_status
    DROP CONSTRAINT IF EXISTS test_status_project_id_fkey;

ALTER TABLE test_status
    ALTER COLUMN project_id DROP NOT NULL;

ALTER TABLE test_status
    DROP COLUMN IF EXISTS project_id;

ALTER TABLE tests
    RENAME COLUMN status_id TO test_status;

ALTER SEQUENCE IF EXISTS test_status_id_seq
    OWNED BY NULL;

ALTER TABLE test_status
    RENAME COLUMN tag TO test_st_short_name,
    RENAME COLUMN description TO test_st_description,
    RENAME COLUMN title TO test_st_title,
    RENAME COLUMN id TO test_st_id;

ALTER SEQUENCE IF EXISTS test_status_id_seq
    RENAME TO test_status_test_st_id_seq;
