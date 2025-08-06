-- Add project_id column to verification table
ALTER TABLE verification ADD COLUMN project_id INTEGER;

-- Set default project_id for existing verification records
-- Assuming project_id 1 (Space project) as default
UPDATE verification SET project_id = 1 WHERE project_id IS NULL;

-- Make project_id NOT NULL after setting default values
ALTER TABLE verification ALTER COLUMN project_id SET NOT NULL;

-- Add foreign key constraint
ALTER TABLE verification ADD CONSTRAINT fk_verification_project 
    FOREIGN KEY (project_id) REFERENCES projects(project_id);
