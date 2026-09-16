-- Revert to namespace-scoped uniqueness (owner/group + slug).
ALTER TABLE projects
    DROP CONSTRAINT IF EXISTS projects_slug_unique;

CREATE UNIQUE INDEX idx_projects_owner_slug_unique
    ON projects (owner_id, slug)
    WHERE group_id IS NULL;

CREATE UNIQUE INDEX idx_projects_group_slug_unique
    ON projects (group_id, slug)
    WHERE group_id IS NOT NULL;
