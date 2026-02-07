DROP INDEX IF EXISTS idx_requirement_versions_search_vector ON requirement_versions;
DROP TRIGGER IF EXISTS requirement_versions_search_vector_trigger ON requirement_versions;
DROP FUNCTION IF EXISTS requirement_versions_search_vector_update();
ALTER TABLE requirement_versions DROP COLUMN IF EXISTS search_vector;
