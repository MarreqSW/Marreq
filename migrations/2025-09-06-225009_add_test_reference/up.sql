-- Add test_reference column to tests table
ALTER TABLE tests ADD COLUMN test_reference VARCHAR NOT NULL DEFAULT 'TEST-0';

-- Update existing tests with proper references
UPDATE tests SET test_reference = 'TEST-' || test_id::text WHERE test_reference = 'TEST-0';