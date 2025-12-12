-- Make project status column non-nullable with default value 'active'

-- Ensure all NULL values are set to 'active' (should already be done, but just in case)
UPDATE projects
SET status = 'active'
WHERE status IS NULL;

-- Make the column NOT NULL with a default value
ALTER TABLE projects
    ALTER COLUMN status SET NOT NULL,
    ALTER COLUMN status SET DEFAULT 'active';
