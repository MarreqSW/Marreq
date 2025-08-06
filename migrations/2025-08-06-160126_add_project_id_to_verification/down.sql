-- Remove foreign key constraint
ALTER TABLE verification DROP CONSTRAINT IF EXISTS fk_verification_project;

-- Remove project_id column
ALTER TABLE verification DROP COLUMN IF EXISTS project_id;
