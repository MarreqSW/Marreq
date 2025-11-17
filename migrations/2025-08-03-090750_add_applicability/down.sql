-- Remove foreign key constraint
ALTER TABLE requirements DROP CONSTRAINT IF EXISTS fk_requirements_applicability;

-- Remove applicability column from requirements table
ALTER TABLE requirements DROP COLUMN IF EXISTS applicability_id;

-- Drop applicability table
DROP TABLE IF EXISTS applicability;
