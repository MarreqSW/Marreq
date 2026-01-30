-- Remove embedding index queue
DROP TABLE IF EXISTS embedding_index_queue;

-- Remove full-text search infrastructure
DROP TRIGGER IF EXISTS requirements_search_vector_trigger ON requirements;
DROP FUNCTION IF EXISTS requirements_search_vector_update();
DROP INDEX IF EXISTS idx_requirements_search_vector;
ALTER TABLE requirements DROP COLUMN IF EXISTS search_vector;

-- Remove embeddings table and indexes
DROP INDEX IF EXISTS idx_requirement_embeddings_vector_hnsw;
DROP INDEX IF EXISTS idx_requirement_embeddings_project_id;
DROP TABLE IF EXISTS requirement_embeddings;

-- Note: We don't drop the vector extension as it may be used by other tables
