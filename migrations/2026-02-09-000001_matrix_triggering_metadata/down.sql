-- Revert matrix triggering metadata columns

ALTER TABLE matrix
    DROP COLUMN IF EXISTS triggering_version_id,
    DROP COLUMN IF EXISTS triggering_user_id;
