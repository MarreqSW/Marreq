-- Wire up the missing foreign key relationships introduced by the schema changes.

ALTER TABLE requirement_status
    ADD COLUMN project_id INTEGER;

UPDATE requirement_status rs
SET project_id = sub.project_id
FROM (
    SELECT req_current_status AS status_id, MIN(project_id) AS project_id
    FROM requirements
    GROUP BY req_current_status
) sub
WHERE rs.req_st_id = sub.status_id;

UPDATE requirement_status
SET project_id = COALESCE(
        project_id,
        (SELECT MIN(project_id) FROM projects)
    );

ALTER TABLE requirement_status
    ALTER COLUMN project_id SET NOT NULL;

ALTER TABLE requirement_status
    ADD CONSTRAINT requirement_status_project_id_fkey
        FOREIGN KEY (project_id) REFERENCES projects(project_id)
        ON DELETE CASCADE;

ALTER TABLE projects
    ADD CONSTRAINT projects_status_id_fkey
        FOREIGN KEY (status_id) REFERENCES project_status(id)
        ON DELETE SET NULL;
