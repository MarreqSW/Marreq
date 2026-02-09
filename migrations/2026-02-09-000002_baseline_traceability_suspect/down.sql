-- Revert baseline_traceability suspect columns

ALTER TABLE baseline_traceability
    DROP COLUMN IF EXISTS suspect,
    DROP COLUMN IF EXISTS suspect_at,
    DROP COLUMN IF EXISTS suspect_reason;
