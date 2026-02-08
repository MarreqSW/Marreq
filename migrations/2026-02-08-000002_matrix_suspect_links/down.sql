-- Revert suspect links columns from matrix

DROP INDEX IF EXISTS idx_matrix_suspect;

ALTER TABLE matrix
    DROP COLUMN IF EXISTS suspect,
    DROP COLUMN IF EXISTS suspect_at,
    DROP COLUMN IF EXISTS suspect_reason,
    DROP COLUMN IF EXISTS cleared_by,
    DROP COLUMN IF EXISTS cleared_at;
