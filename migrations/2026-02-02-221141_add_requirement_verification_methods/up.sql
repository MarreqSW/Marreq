-- Junction table: requirements can have multiple verification methods
CREATE TABLE requirement_verification_methods (
    requirement_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    verification_method_id INTEGER NOT NULL REFERENCES verification(id) ON DELETE CASCADE,
    PRIMARY KEY (requirement_id, verification_method_id)
);

-- Migrate existing single verification_method_id into the junction table
INSERT INTO requirement_verification_methods (requirement_id, verification_method_id)
SELECT id, verification_method_id FROM requirements;

-- Remove the single verification method column from requirements
ALTER TABLE requirements DROP COLUMN verification_method_id;

-- Index for filtering requirements by verification method
CREATE INDEX idx_req_verification_methods_verification_id
ON requirement_verification_methods(verification_method_id);
