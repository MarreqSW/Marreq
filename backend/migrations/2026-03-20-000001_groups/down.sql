DROP INDEX IF EXISTS idx_group_members_user_id;
DROP INDEX IF EXISTS idx_projects_group_id;
ALTER TABLE projects DROP COLUMN IF EXISTS group_id;
DROP TABLE IF EXISTS group_members;
DROP TABLE IF EXISTS groups;
