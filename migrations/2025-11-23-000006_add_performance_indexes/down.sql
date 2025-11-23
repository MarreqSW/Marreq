-- Drop the composite indexes that were created in the up migration.

DROP INDEX IF EXISTS idx_matrix_project_req;
DROP INDEX IF EXISTS idx_tests_project_status;
DROP INDEX IF EXISTS idx_requirements_project_status;
