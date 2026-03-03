-- Revert cross-project integrity triggers
DROP TRIGGER IF EXISTS cfv_project_consistency  ON custom_field_values;
DROP TRIGGER IF EXISTS rvvm_project_consistency ON requirement_version_verification_methods;
DROP TRIGGER IF EXISTS rvl_project_consistency  ON requirement_version_links;
DROP TRIGGER IF EXISTS matrix_project_consistency ON matrix;

DROP FUNCTION IF EXISTS check_cfv_project_consistency();
DROP FUNCTION IF EXISTS check_rvvm_project_consistency();
DROP FUNCTION IF EXISTS check_rvl_project_consistency();
DROP FUNCTION IF EXISTS check_matrix_project_consistency();
