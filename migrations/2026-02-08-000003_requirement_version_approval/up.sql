-- =============================================================================
-- Approval workflow for requirement versions
-- =============================================================================
-- approval_state: draft -> reviewed -> approved (enforced at API layer).
-- approved_by / approved_at set when transitioning to approved.
-- Existing status_id (requirement_status) is unchanged.
-- =============================================================================

-- 1. Add approval columns to requirement_versions
ALTER TABLE requirement_versions
    ADD COLUMN approval_state VARCHAR NOT NULL DEFAULT 'draft',
    ADD COLUMN approved_by INTEGER NULL REFERENCES users(id),
    ADD COLUMN approved_at TIMESTAMP NULL;

-- 2. Constraint: only allow known enum values
ALTER TABLE requirement_versions
    ADD CONSTRAINT requirement_versions_approval_state_check
    CHECK (approval_state IN ('draft', 'reviewed', 'approved'));

-- 3. Index for baseline/listing by approval (e.g. "only approved")
CREATE INDEX idx_requirement_versions_approval_state ON requirement_versions(approval_state);

COMMENT ON COLUMN requirement_versions.approval_state IS 'Workflow: draft | reviewed | approved';
COMMENT ON COLUMN requirement_versions.approved_by IS 'User who approved this version (set when approval_state = approved)';
COMMENT ON COLUMN requirement_versions.approved_at IS 'When this version was approved';
