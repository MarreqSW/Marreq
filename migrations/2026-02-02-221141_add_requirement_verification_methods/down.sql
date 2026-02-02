-- Restore verification_method_id column on requirements
ALTER TABLE requirements ADD COLUMN verification_method_id INTEGER;

-- Populate from junction table (pick first verification method per requirement)
UPDATE requirements r
SET verification_method_id = (
    SELECT verification_method_id
    FROM requirement_verification_methods rvm
    WHERE rvm.requirement_id = r.id
    LIMIT 1
);

-- Default any remaining NULLs to 1 (e.g. requirements added during migration with no links)
UPDATE requirements SET verification_method_id = 1 WHERE verification_method_id IS NULL;
ALTER TABLE requirements ALTER COLUMN verification_method_id SET NOT NULL;

-- Add FK (optional; can reference verification(id))
ALTER TABLE requirements
ADD CONSTRAINT requirements_verification_method_id_fkey
FOREIGN KEY (verification_method_id) REFERENCES verification(id);

-- Drop junction table
DROP TABLE requirement_verification_methods;
