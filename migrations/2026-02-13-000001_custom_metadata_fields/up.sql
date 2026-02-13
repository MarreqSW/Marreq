-- =============================================================================
-- Project-scoped custom metadata fields for requirements
-- =============================================================================
-- Definitions live per project; values are stored per requirement_version
-- so they are part of version history. No schema change needed when adding
-- new field definitions.
-- =============================================================================

-- Custom field definition (project-scoped)
-- field_type: text | enum | boolean | number
CREATE TABLE custom_field_definitions (
    id SERIAL PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    label VARCHAR(255) NOT NULL,
    field_type VARCHAR(20) NOT NULL CHECK (field_type IN ('text', 'enum', 'boolean', 'number')),
    enum_values JSONB,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_custom_field_definitions_project ON custom_field_definitions(project_id);

-- Values stored per requirement version (one row per version per field)
CREATE TABLE custom_field_values (
    requirement_version_id INTEGER NOT NULL REFERENCES requirement_versions(id) ON DELETE CASCADE,
    custom_field_definition_id INTEGER NOT NULL REFERENCES custom_field_definitions(id) ON DELETE CASCADE,
    value TEXT,
    PRIMARY KEY (requirement_version_id, custom_field_definition_id)
);

CREATE INDEX idx_custom_field_values_version ON custom_field_values(requirement_version_id);
CREATE INDEX idx_custom_field_values_definition ON custom_field_values(custom_field_definition_id);

ALTER TABLE custom_field_definitions
    ADD CONSTRAINT custom_field_definitions_enum_values_for_enum_type
    CHECK (
        (field_type <> 'enum') OR (enum_values IS NOT NULL AND jsonb_typeof(enum_values) = 'array')
    );
