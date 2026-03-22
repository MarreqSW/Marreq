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
