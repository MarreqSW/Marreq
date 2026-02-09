-- =============================================================================
-- Matrix triggering metadata (who/what caused suspect)
-- =============================================================================
-- When a link is marked suspect, record the requirement version and user that
-- triggered it (edit or approval transition) for audit and impacted-tests queries.
-- =============================================================================

ALTER TABLE matrix
    ADD COLUMN triggering_version_id INTEGER NULL REFERENCES requirement_versions(id) ON DELETE SET NULL,
    ADD COLUMN triggering_user_id INTEGER NULL REFERENCES users(id) ON DELETE SET NULL;

COMMENT ON COLUMN matrix.triggering_version_id IS 'Requirement version that caused the link to be marked suspect';
COMMENT ON COLUMN matrix.triggering_user_id IS 'User who triggered the change (edit or approval transition)';
