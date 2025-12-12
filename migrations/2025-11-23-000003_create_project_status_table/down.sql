-- Revert to the textual project_status column on projects.

ALTER TABLE projects
    ADD COLUMN project_status VARCHAR(50);

UPDATE projects p
SET project_status = ps.name
FROM project_status ps
WHERE p.status_id = ps.id;

ALTER TABLE projects
    DROP COLUMN status_id;

DROP TABLE project_status;
