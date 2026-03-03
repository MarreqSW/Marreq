-- Performance indexes for project-scoped and filter-heavy queries.
-- See: status_service (list by project), requirement_analytics_service,
-- diesel_repo list requirements with filters, custom_field_values filter by value.

-- Status tables: project-scoped lookups
CREATE INDEX idx_requirement_status_project_id ON requirement_status(project_id);
CREATE INDEX idx_test_status_project_id ON test_status(project_id);

-- Requirement versions: filter by status, category, applicability
CREATE INDEX idx_requirement_versions_status_id ON requirement_versions(status_id);
CREATE INDEX idx_requirement_versions_category_id ON requirement_versions(category_id);
CREATE INDEX idx_requirement_versions_applicability_id ON requirement_versions(applicability_id);

-- Verification junction: reverse lookup by verification_method_id
CREATE INDEX idx_rvvm_verification_method_id ON requirement_version_verification_methods(verification_method_id, requirement_version_id);

-- Custom field values: filter by definition and value
CREATE INDEX idx_custom_field_values_definition_value ON custom_field_values(custom_field_definition_id, value);
