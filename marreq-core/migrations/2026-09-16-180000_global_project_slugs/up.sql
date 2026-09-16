-- Restore globally unique project slugs so URLs can be /<slug>/... without a
-- user or group namespace prefix.

-- Keep the lowest id for each slug; suffix later duplicates with -<id>.
UPDATE projects p
SET slug = p.slug || '-' || p.id::text
WHERE p.id IN (
    SELECT id
    FROM (
        SELECT id,
               ROW_NUMBER() OVER (PARTITION BY slug ORDER BY id) AS rn
        FROM projects
    ) ranked
    WHERE ranked.rn > 1
);

DROP INDEX IF EXISTS idx_projects_owner_slug_unique;
DROP INDEX IF EXISTS idx_projects_group_slug_unique;

ALTER TABLE projects
    DROP CONSTRAINT IF EXISTS projects_slug_unique;

ALTER TABLE projects
    ADD CONSTRAINT projects_slug_unique UNIQUE (slug);
