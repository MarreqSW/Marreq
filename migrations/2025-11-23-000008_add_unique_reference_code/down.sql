-- Remove the reference code uniqueness constraints.

ALTER TABLE requirements
    DROP CONSTRAINT IF EXISTS requirements_reference_code_unique;

ALTER TABLE tests
    DROP CONSTRAINT IF EXISTS tests_reference_code_unique;
