-- Introduce a dedicated table for project statuses and move the textual column to an FK.

CREATE TABLE project_status (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL,
    description TEXT,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE projects
    ADD COLUMN status_id INTEGER;

WITH distinct_status AS (
    SELECT DISTINCT project_status AS name
    FROM projects
    WHERE project_status IS NOT NULL
)
INSERT INTO project_status (name, description)
SELECT name, 'Migrated from projects table'
FROM distinct_status
WHERE name <> '';

UPDATE projects p
SET status_id = ps.id
FROM project_status ps
WHERE p.project_status IS NOT NULL
  AND p.project_status = ps.name;

ALTER TABLE projects
    DROP COLUMN project_status;
