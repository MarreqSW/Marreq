-- Changes:
-- 1) Create table project_members (if it does not exist)
-- 2) Backfill from users.project_id -> project_members
-- 3) Drop obsolete columns from users: user_level, project_id

SET lock_timeout = '5s';
SET statement_timeout = '5min';

BEGIN;

-- 1) Create the project_members table
CREATE TABLE IF NOT EXISTS project_members (
    project_id INTEGER NOT NULL REFERENCES projects(project_id) ON DELETE CASCADE,
    user_id    INTEGER NOT NULL REFERENCES users(user_id)     ON DELETE CASCADE,
    role       INTEGER NOT NULL DEFAULT 2, -- 1=Owner, 2=Manager, 3=Contributor, 4=Viewer
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (project_id, user_id)
);

-- 2) Backfill: migrate memberships from the old users.project_id
--    Avoid duplicates if re-run.
INSERT INTO project_members (project_id, user_id, role, created_at, updated_at)
SELECT
    u.project_id,
    u.user_id,
    CASE WHEN u.is_admin IS TRUE THEN 1 ELSE 2 END AS role,
    u.user_creation_date,
    CURRENT_TIMESTAMP
FROM users u
WHERE u.project_id IS NOT NULL
  AND NOT EXISTS (
      SELECT 1
      FROM project_members pm
      WHERE pm.project_id = u.project_id
        AND pm.user_id    = u.user_id
  );

-- 3) Drop old columns from the users table
ALTER TABLE users
  DROP COLUMN IF EXISTS user_level,
  DROP COLUMN IF EXISTS project_id;

COMMIT;

-- Checks (run separately if needed):
-- \echo 'Users: current columns'
-- SELECT column_name FROM information_schema.columns
--   WHERE table_schema='public' AND table_name='users' ORDER BY 1;
-- \echo 'Membership count'
-- SELECT COUNT(*) AS memberships FROM project_members;
