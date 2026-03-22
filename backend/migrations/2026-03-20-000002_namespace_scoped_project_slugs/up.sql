-- "admin" is reserved as a URL namespace segment (SPA paths like /admin/users). Sample data used
-- username admin; rename so the checks below succeed on existing databases.
UPDATE users
SET username = 'sysadmin'
WHERE lower(username) = 'admin'
  AND NOT EXISTS (
      SELECT 1 FROM users u2 WHERE lower(u2.username) = 'sysadmin' AND u2.id <> users.id
  );

UPDATE groups
SET slug = 'administrators'
WHERE slug = 'admin'
  AND NOT EXISTS (SELECT 1 FROM groups g2 WHERE g2.slug = 'administrators' AND g2.id <> groups.id);

-- Namespace URLs require every project to belong to an owner or a group. Legacy rows may have both NULL.
UPDATE projects
SET owner_id = (SELECT id FROM users WHERE is_admin = true ORDER BY id LIMIT 1)
WHERE group_id IS NULL
  AND owner_id IS NULL
  AND EXISTS (SELECT 1 FROM users WHERE is_admin = true);

UPDATE projects
SET owner_id = (SELECT id FROM users ORDER BY id LIMIT 1)
WHERE group_id IS NULL
  AND owner_id IS NULL
  AND EXISTS (SELECT 1 FROM users LIMIT 1);

DO $$
DECLARE
    conflicting_namespace TEXT;
BEGIN
    SELECT LOWER(username)
    INTO conflicting_namespace
    FROM users
    WHERE LOWER(username) IN (
        'admin',
        'api',
        'cache',
        'change_password',
        'cleanup_logs',
        'error',
        'export_logs',
        'groups',
        'log_analytics',
        'login',
        'logout',
        'logs',
        'new_project',
        'profile',
        'projects',
        'static',
        'status',
        'user'
    )
    LIMIT 1;

    IF conflicting_namespace IS NOT NULL THEN
        RAISE EXCEPTION
            E'Cannot enable namespace-style project URLs: user namespace "%" conflicts with a reserved root path.',
            conflicting_namespace;
    END IF;

    SELECT slug
    INTO conflicting_namespace
    FROM groups
    WHERE slug IN (
        'admin',
        'api',
        'cache',
        'change_password',
        'cleanup_logs',
        'error',
        'export_logs',
        'groups',
        'log_analytics',
        'login',
        'logout',
        'logs',
        'new_project',
        'profile',
        'projects',
        'static',
        'status',
        'user'
    )
    LIMIT 1;

    IF conflicting_namespace IS NOT NULL THEN
        RAISE EXCEPTION
            E'Cannot enable namespace-style project URLs: group namespace "%" conflicts with a reserved root path.',
            conflicting_namespace;
    END IF;

    SELECT LOWER(u.username)
    INTO conflicting_namespace
    FROM users u
    INNER JOIN groups g
        ON LOWER(u.username) = g.slug
    LIMIT 1;

    IF conflicting_namespace IS NOT NULL THEN
        RAISE EXCEPTION
            E'Cannot enable namespace-style project URLs: namespace "%" is used by both a user and a group.',
            conflicting_namespace;
    END IF;

    SELECT p.id::TEXT
    INTO conflicting_namespace
    FROM projects p
    WHERE p.group_id IS NULL
      AND p.owner_id IS NULL
    LIMIT 1;

    IF conflicting_namespace IS NOT NULL THEN
        RAISE EXCEPTION
            E'Cannot enable namespace-style project URLs: project id % has neither owner_id nor group_id.',
            conflicting_namespace;
    END IF;
END $$;

ALTER TABLE projects
DROP CONSTRAINT IF EXISTS projects_slug_unique;

CREATE UNIQUE INDEX idx_projects_owner_slug_unique
    ON projects (owner_id, slug)
    WHERE group_id IS NULL;

CREATE UNIQUE INDEX idx_projects_group_slug_unique
    ON projects (group_id, slug)
    WHERE group_id IS NOT NULL;
