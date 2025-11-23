-- Normalize the test status table and rename columns to match the new schema.

-- Rename the columns on the `test_status` table to the diesel-friendly names.
ALTER TABLE test_status
    RENAME COLUMN test_st_id TO id;

ALTER SEQUENCE IF EXISTS test_status_test_st_id_seq
    RENAME TO test_status_id_seq;

ALTER TABLE test_status
    RENAME COLUMN test_st_title TO title,
    RENAME COLUMN test_st_description TO description,
    RENAME COLUMN test_st_short_name TO tag;

-- Ensure the auto increment still targets the renamed primary key.
ALTER SEQUENCE IF EXISTS test_status_id_seq
    OWNED BY test_status.id;

-- Rename the foreign key column on tests so it matches the new status naming.
ALTER TABLE tests
    RENAME COLUMN test_status TO status_id;

-- Track which project each status entry belongs to.
ALTER TABLE test_status
    ADD COLUMN project_id INTEGER;

-- Attempt to infer the owning project from the tests that use each status.
UPDATE test_status ts
SET project_id = sub.project_id
FROM (
    SELECT status_id, MIN(project_id) AS project_id
    FROM tests
    GROUP BY status_id
) sub
WHERE ts.id = sub.status_id;

-- Fallback to the smallest project id when a status is unused.
UPDATE test_status
SET project_id = COALESCE(
        project_id,
        (SELECT MIN(project_id) FROM projects)
    );

ALTER TABLE test_status
    ALTER COLUMN project_id SET NOT NULL;

ALTER TABLE test_status
    ADD CONSTRAINT test_status_project_id_fkey
        FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON DELETE CASCADE;
