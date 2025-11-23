-- Remove the foreign key relationships that were added in the up migration.

ALTER TABLE projects
    DROP CONSTRAINT IF EXISTS projects_status_id_fkey;

ALTER TABLE requirement_status
    DROP CONSTRAINT IF EXISTS requirement_status_project_id_fkey;

ALTER TABLE requirement_status
    ALTER COLUMN project_id DROP NOT NULL;

ALTER TABLE requirement_status
    DROP COLUMN IF EXISTS project_id;
