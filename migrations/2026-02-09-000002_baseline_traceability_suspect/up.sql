-- =============================================================================
-- Baseline traceability: capture suspect state at baseline time
-- =============================================================================
-- When a baseline is created, snapshot whether each traceability link was
-- suspect at that time (for audit and display).
-- =============================================================================

ALTER TABLE baseline_traceability
    ADD COLUMN suspect BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN suspect_at TIMESTAMP NULL,
    ADD COLUMN suspect_reason TEXT NULL;

COMMENT ON COLUMN baseline_traceability.suspect IS 'Whether the link was marked suspect at baseline creation time';
COMMENT ON COLUMN baseline_traceability.suspect_at IS 'When the link was marked suspect (at baseline time)';
COMMENT ON COLUMN baseline_traceability.suspect_reason IS 'Reason the link was suspect (at baseline time)';
