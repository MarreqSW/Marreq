-- Remove the non-empty string constraints.

ALTER TABLE requirements
    DROP CONSTRAINT IF EXISTS requirements_title_not_blank;

ALTER TABLE tests
    DROP CONSTRAINT IF EXISTS tests_name_not_blank;

ALTER TABLE projects
    DROP CONSTRAINT IF EXISTS projects_name_not_blank;
