-- =============================================================================
-- Suspect links (change impact analysis)
-- =============================================================================
-- When a requirement changes (new version), downstream traceability links are
-- marked suspect until explicitly reviewed and cleared.
-- =============================================================================

ALTER TABLE matrix
    ADD COLUMN suspect BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN suspect_at TIMESTAMP,
    ADD COLUMN suspect_reason TEXT,
    ADD COLUMN cleared_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    ADD COLUMN cleared_at TIMESTAMP;

COMMENT ON COLUMN matrix.suspect IS 'True when the link needs re-review (e.g. requirement updated)';
COMMENT ON COLUMN matrix.suspect_at IS 'When the link was marked suspect';
COMMENT ON COLUMN matrix.suspect_reason IS 'Reason (e.g. Requirement updated)';
COMMENT ON COLUMN matrix.cleared_by IS 'User who cleared the suspect flag';
COMMENT ON COLUMN matrix.cleared_at IS 'When the suspect flag was cleared';

CREATE INDEX idx_matrix_suspect ON matrix(suspect) WHERE suspect = true;
