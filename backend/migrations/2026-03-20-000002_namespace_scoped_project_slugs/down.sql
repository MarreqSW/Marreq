DROP INDEX IF EXISTS idx_projects_group_slug_unique;
DROP INDEX IF EXISTS idx_projects_owner_slug_unique;

DO $$
BEGIN
    IF EXISTS (
        SELECT slug
        FROM projects
        GROUP BY slug
        HAVING COUNT(*) > 1
    ) THEN
        RAISE EXCEPTION
            E'Cannot restore global project slug uniqueness: duplicate slugs already exist across namespaces.';
    END IF;
END $$;

ALTER TABLE projects
ADD CONSTRAINT projects_slug_unique UNIQUE (slug);
