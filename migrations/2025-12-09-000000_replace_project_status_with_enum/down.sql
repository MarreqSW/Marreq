-- Revert to using project_status table with foreign key
-- This is the reverse of the up migration

-- Recreate the project_status table
CREATE TABLE project_status (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL,
    description TEXT,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Populate with standard statuses
INSERT INTO project_status (name, description) VALUES
    ('Active', 'The project is currently active and in progress'),
    ('Completed', 'The project has been completed'),
    ('On Hold', 'The project is temporarily on hold'),
    ('Cancelled', 'The project has been cancelled');

-- Add status_id column back to projects
ALTER TABLE projects
    ADD COLUMN status_id INTEGER;

-- Migrate data from status column to status_id
UPDATE projects p
SET status_id = ps.id
FROM project_status ps
WHERE p.status IS NOT NULL
  AND (
    (p.status = 'active' AND ps.name = 'Active') OR
    (p.status = 'completed' AND ps.name = 'Completed') OR
    (p.status = 'on_hold' AND ps.name = 'On Hold') OR
    (p.status = 'cancelled' AND ps.name = 'Cancelled')
  );

-- Set default for any NULL values (should not happen but be safe)
UPDATE projects p
SET status_id = (SELECT id FROM project_status WHERE name = 'Active' LIMIT 1)
WHERE status_id IS NULL;

-- Drop the status column
ALTER TABLE projects
    DROP COLUMN status;

-- Add foreign key constraint
ALTER TABLE projects
    ADD CONSTRAINT projects_status_id_fkey
    FOREIGN KEY (status_id) REFERENCES project_status(id);
