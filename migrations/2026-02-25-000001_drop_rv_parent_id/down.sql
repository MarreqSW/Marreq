-- Restore parent_id column on requirement_versions (nullable, no FK).
ALTER TABLE requirement_versions ADD COLUMN parent_id INTEGER;

-- Re-create the legacy index.
CREATE INDEX IF NOT EXISTS idx_requirements_parent ON requirement_versions(parent_id);

-- Backfill parent_id from requirement_version_links (first DERIVES_FROM link per source).
UPDATE requirement_versions rv
SET parent_id = sub.parent_req_id
FROM (
    SELECT DISTINCT ON (rvl.source_version_id)
           rvl.source_version_id,
           r_parent.id AS parent_req_id
    FROM requirement_version_links rvl
    JOIN requirement_versions rv_target ON rv_target.id = rvl.target_version_id
    JOIN requirements r_parent ON r_parent.id = rv_target.requirement_id
    ORDER BY rvl.source_version_id, rvl.id
) sub
WHERE rv.id = sub.source_version_id;
