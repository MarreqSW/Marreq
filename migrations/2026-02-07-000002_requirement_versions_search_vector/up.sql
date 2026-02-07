-- Full-text search for requirement_versions (used by lexical search after versioning)
-- Content lives in requirement_versions; search_vector is maintained per version.

ALTER TABLE requirement_versions
    ADD COLUMN IF NOT EXISTS search_vector tsvector;

-- Populate from current version content (title, description, justification) and requirement stable_code
CREATE OR REPLACE FUNCTION requirement_versions_search_vector_update() RETURNS trigger AS $$
DECLARE
    stable_code_val VARCHAR;
BEGIN
    SELECT COALESCE(r.stable_code, '') INTO stable_code_val
    FROM requirements r WHERE r.id = NEW.requirement_id;
    NEW.search_vector :=
        setweight(to_tsvector('english', COALESCE(stable_code_val, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.description, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(NEW.justification, '')), 'C');
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS requirement_versions_search_vector_trigger ON requirement_versions;
CREATE TRIGGER requirement_versions_search_vector_trigger
    BEFORE INSERT OR UPDATE OF title, description, justification, requirement_id
    ON requirement_versions
    FOR EACH ROW
    EXECUTE FUNCTION requirement_versions_search_vector_update();

-- Backfill existing rows
UPDATE requirement_versions rv
SET search_vector = (
    setweight(to_tsvector('english', COALESCE(r.stable_code, '')), 'A') ||
    setweight(to_tsvector('english', COALESCE(rv.title, '')), 'A') ||
    setweight(to_tsvector('english', COALESCE(rv.description, '')), 'B') ||
    setweight(to_tsvector('english', COALESCE(rv.justification, '')), 'C')
)
FROM requirements r
WHERE r.id = rv.requirement_id AND rv.search_vector IS NULL;

CREATE INDEX IF NOT EXISTS idx_requirement_versions_search_vector
    ON requirement_versions USING gin(search_vector);
