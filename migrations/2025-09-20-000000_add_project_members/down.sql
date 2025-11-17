-- Changes:
-- 1) Add back users.user_level and users.project_id
-- 2) Restore users.project_id from project_members using role priority
-- 3) Optionally drop project_members

SET lock_timeout = '5s';
SET statement_timeout = '5min';

BEGIN;

-- 1) Add back the dropped columns
ALTER TABLE users
  ADD COLUMN IF NOT EXISTS user_level INTEGER NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS project_id INTEGER;

-- 2) Rehydrate project_id choosing the “best” membership for each user
WITH ranked AS (
  SELECT
    pm.id,
    pm.project_id,
    pm.role,
    ROW_NUMBER() OVER (
      PARTITION BY pm.id
      ORDER BY
        CASE pm.role
          WHEN 1 THEN 1  -- Owner
          WHEN 2 THEN 2  -- Manager
          WHEN 3 THEN 3  -- Contributor
          WHEN 4 THEN 4  -- Viewer
          ELSE 5
        END,
        pm.project_id ASC
    ) AS rn
  FROM project_members pm
)
UPDATE users u
SET project_id = r.project_id
FROM ranked r
WHERE r.id = u.id
  AND r.rn = 1;

-- 3) Optionally drop the project_members table if you want a full rollback
-- DROP TABLE IF EXISTS project_members;

COMMIT;

-- Verification:
-- SELECT id, project_id FROM users ORDER BY id LIMIT 50;
