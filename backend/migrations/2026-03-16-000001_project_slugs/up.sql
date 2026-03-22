ALTER TABLE projects
ADD COLUMN slug VARCHAR(255);

WITH base_slugs AS (
    SELECT
        id,
        COALESCE(
            NULLIF(
                BTRIM(
                    REGEXP_REPLACE(LOWER(name), '[^a-z0-9]+', '-', 'g'),
                    '-'
                ),
                ''
            ),
            'project'
        ) AS base_slug
    FROM projects
),
numbered_slugs AS (
    SELECT
        id,
        base_slug,
        ROW_NUMBER() OVER (PARTITION BY base_slug ORDER BY id) AS occurrence
    FROM base_slugs
)
UPDATE projects
SET slug = CASE
    WHEN numbered_slugs.occurrence = 1 THEN LEFT(numbered_slugs.base_slug, 255)
    ELSE
        LEFT(
            numbered_slugs.base_slug,
            255 - LENGTH('-' || numbered_slugs.occurrence::TEXT)
        ) || '-' || numbered_slugs.occurrence::TEXT
END
FROM numbered_slugs
WHERE projects.id = numbered_slugs.id;

ALTER TABLE projects
ALTER COLUMN slug SET NOT NULL;

ALTER TABLE projects
ADD CONSTRAINT projects_slug_unique UNIQUE (slug);
