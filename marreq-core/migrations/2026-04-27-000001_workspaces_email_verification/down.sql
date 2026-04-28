DROP TABLE IF EXISTS email_tokens;
DROP INDEX IF EXISTS workspaces_one_personal_per_user_idx;
DROP INDEX IF EXISTS workspaces_owner_user_id_idx;
DROP TABLE IF EXISTS workspaces;
ALTER TABLE users DROP COLUMN IF EXISTS email_verified;
