-- Add reference_code column to tests table
ALTER TABLE tests ADD COLUMN reference_code VARCHAR NOT NULL DEFAULT 'TEST-0';

-- Update existing tests with proper references
UPDATE tests SET reference_code = 'TEST-' || id::text WHERE reference_code = 'TEST-0';