-- Enforce globally unique reference codes for requirements and tests.

ALTER TABLE requirements
    ADD CONSTRAINT requirements_reference_code_unique UNIQUE (req_reference);

ALTER TABLE tests
    ADD CONSTRAINT tests_reference_code_unique UNIQUE (test_reference);
