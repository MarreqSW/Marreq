-- Remove foreign key constraint
ALTER TABLE requirements DROP CONSTRAINT IF EXISTS fk_requirements_applicability;

-- Remove applicability column from requirements table
ALTER TABLE requirements DROP COLUMN IF EXISTS req_applicability;

-- Drop applicability table
DROP TABLE IF EXISTS applicability;
