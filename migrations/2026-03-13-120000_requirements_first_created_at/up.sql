-- Add first_created_at so Requirement view can show stable creation_date across version edits.
-- Default allows insert without the column; backfill and update-on-create set the real value.
ALTER TABLE requirements
ADD COLUMN first_created_at TIMESTAMP NOT NULL DEFAULT now();

-- Backfill from earliest requirement_version per requirement.
UPDATE requirements r
SET first_created_at = (
    SELECT MIN(rv.created_at)
    FROM requirement_versions rv
    WHERE rv.requirement_id = r.id
);
