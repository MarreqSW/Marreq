-- Prevent cycles in the requirement version DAG.
-- Edges are stored as source_version_id -> target_version_id, where source is the child
-- and target is the parent/upstream requirement version.

CREATE OR REPLACE FUNCTION prevent_requirement_version_link_cycles()
RETURNS trigger AS $$
DECLARE
    cycle_found boolean;
BEGIN
    IF NEW.source_version_id = NEW.target_version_id THEN
        RAISE EXCEPTION '[requirement_version_links_cycle] source_version_id and target_version_id must differ';
    END IF;

    WITH RECURSIVE ancestors(version_id) AS (
        SELECT rvl.target_version_id
        FROM requirement_version_links rvl
        WHERE rvl.source_version_id = NEW.target_version_id
          AND rvl.project_id = NEW.project_id
          AND (TG_OP <> 'UPDATE' OR rvl.id <> OLD.id)

        UNION

        SELECT rvl.target_version_id
        FROM requirement_version_links rvl
        INNER JOIN ancestors a ON rvl.source_version_id = a.version_id
        WHERE rvl.project_id = NEW.project_id
          AND (TG_OP <> 'UPDATE' OR rvl.id <> OLD.id)
    )
    SELECT EXISTS (
        SELECT 1
        FROM ancestors
        WHERE version_id = NEW.source_version_id
    ) INTO cycle_found;

    IF cycle_found THEN
        RAISE EXCEPTION '[requirement_version_links_cycle] creating this link would introduce a cycle in requirement version links';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_prevent_requirement_version_link_cycles ON requirement_version_links;

CREATE TRIGGER trg_prevent_requirement_version_link_cycles
BEFORE INSERT OR UPDATE OF source_version_id, target_version_id, project_id
ON requirement_version_links
FOR EACH ROW
EXECUTE FUNCTION prevent_requirement_version_link_cycles();
