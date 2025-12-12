-- Replace project_status table with a simple VARCHAR column using enum values
-- This migration converts the status_id foreign key to a direct status string column

-- Add the new status column
ALTER TABLE projects
    ADD COLUMN status VARCHAR(50);

-- Migrate existing data from status_id to status
UPDATE projects p
SET status = CASE ps.name
    WHEN 'Active' THEN 'active'
    WHEN 'Completed' THEN 'completed'
    WHEN 'On Hold' THEN 'on_hold'
    WHEN 'Cancelled' THEN 'cancelled'
    ELSE 'active'
END
FROM project_status ps
WHERE p.status_id = ps.id;

-- Set default status for any NULL values
UPDATE projects
SET status = 'active'
WHERE status IS NULL;

-- Drop the foreign key constraint and status_id column
ALTER TABLE projects
    DROP CONSTRAINT IF EXISTS projects_status_id_fkey;

ALTER TABLE projects
    DROP COLUMN status_id;

-- Drop the project_status table
DROP TABLE IF EXISTS project_status;
