-- Revert baseline schema.
-- This is intentionally destructive and intended for dev use only.

DROP TRIGGER IF EXISTS requirements_search_vector_trigger ON requirements;
DROP FUNCTION IF EXISTS requirements_search_vector_update();

DROP TABLE IF EXISTS embedding_index_queue CASCADE;
DROP TABLE IF EXISTS requirement_embeddings CASCADE;

DROP TABLE IF EXISTS logs CASCADE;
DROP TABLE IF EXISTS matrix CASCADE;
DROP TABLE IF EXISTS tests CASCADE;
DROP TABLE IF EXISTS requirements CASCADE;
DROP TABLE IF EXISTS verification CASCADE;
DROP TABLE IF EXISTS applicability CASCADE;
DROP TABLE IF EXISTS categories CASCADE;
DROP TABLE IF EXISTS test_status CASCADE;
DROP TABLE IF EXISTS requirement_status CASCADE;
DROP TABLE IF EXISTS project_members CASCADE;
DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS projects CASCADE;

DROP FUNCTION IF EXISTS diesel_manage_updated_at(_tbl regclass);
DROP FUNCTION IF EXISTS diesel_set_updated_at();

DROP EXTENSION IF EXISTS vector;

