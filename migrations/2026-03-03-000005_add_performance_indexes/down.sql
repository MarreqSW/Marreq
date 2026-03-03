-- Revert: drop performance indexes (reverse order of creation)
DROP INDEX IF EXISTS idx_custom_field_values_definition_value;
DROP INDEX IF EXISTS idx_rvvm_verification_method_id;
DROP INDEX IF EXISTS idx_requirement_versions_applicability_id;
DROP INDEX IF EXISTS idx_requirement_versions_category_id;
DROP INDEX IF EXISTS idx_requirement_versions_status_id;
DROP INDEX IF EXISTS idx_test_status_project_id;
DROP INDEX IF EXISTS idx_requirement_status_project_id;
