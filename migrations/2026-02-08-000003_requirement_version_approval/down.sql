DROP INDEX IF EXISTS idx_requirement_versions_approval_state;
ALTER TABLE requirement_versions DROP CONSTRAINT IF EXISTS requirement_versions_approval_state_check;
ALTER TABLE requirement_versions
    DROP COLUMN IF EXISTS approval_state,
    DROP COLUMN IF EXISTS approved_by,
    DROP COLUMN IF EXISTS approved_at;
