-- Prevent cycles in the requirement version DAG.
-- Edges are stored as source_version_id -> target_version_id, where source is the child
-- and target is the parent/upstream requirement version.

CREATE OR REPLACE FUNCTION prevent_requirement_version_link_cycles()
RETURNS trigger AS $$
DECLARE
    cycle_found boolean;
    excluded_link_id integer := NULL;
BEGIN
    IF TG_OP = 'UPDATE' THEN
        excluded_link_id := OLD.id;
    END IF;

    IF NEW.source_version_id = NEW.target_version_id THEN
        RAISE EXCEPTION 'source_version_id and target_version_id must differ'
            USING ERRCODE = '23514',
                  CONSTRAINT = 'requirement_version_links_no_self_link';
    END IF;

    WITH RECURSIVE ancestors(version_id) AS (
        SELECT rvl.target_version_id
        FROM requirement_version_links rvl
        WHERE rvl.source_version_id = NEW.target_version_id
          AND rvl.project_id = NEW.project_id
          AND (excluded_link_id IS NULL OR rvl.id <> excluded_link_id)

        UNION

        SELECT rvl.target_version_id
        FROM requirement_version_links rvl
        INNER JOIN ancestors a ON rvl.source_version_id = a.version_id
        WHERE rvl.project_id = NEW.project_id
          AND (excluded_link_id IS NULL OR rvl.id <> excluded_link_id)
    )
    SELECT EXISTS (
        SELECT 1
        FROM ancestors
        WHERE version_id = NEW.source_version_id
    ) INTO cycle_found;

    IF cycle_found THEN
        RAISE EXCEPTION 'creating this link would introduce a cycle in requirement version links'
            USING ERRCODE = '23514',
                  CONSTRAINT = 'requirement_version_links_no_cycles';
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
