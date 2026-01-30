-- Enable pgvector extension for vector similarity search
CREATE EXTENSION IF NOT EXISTS vector;

-- Create table for storing requirement embeddings
-- Uses requirement_id as primary key with CASCADE delete for automatic cleanup
-- Default dimension 768 for nomic-embed-text (Ollama), can store up to 1024 dims
CREATE TABLE requirement_embeddings (
    requirement_id INTEGER PRIMARY KEY REFERENCES requirements(id) ON DELETE CASCADE,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    embedding vector(1024),  -- Supports most Ollama models (768-1024 dims)
    embedding_model VARCHAR(100) NOT NULL DEFAULT 'nomic-embed-text',
    content_hash VARCHAR(64) NOT NULL,  -- SHA256 hash of embedding source text
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Index for project-scoped queries
CREATE INDEX idx_requirement_embeddings_project_id 
    ON requirement_embeddings(project_id);

-- HNSW index for fast approximate nearest neighbor search using cosine distance
-- m=16: number of bi-directional links (higher = better recall, more memory)
-- ef_construction=64: size of dynamic candidate list during construction
CREATE INDEX idx_requirement_embeddings_vector_hnsw
    ON requirement_embeddings 
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

-- Add full-text search vector column to requirements table
-- This will be populated via trigger for automatic updates
ALTER TABLE requirements 
    ADD COLUMN search_vector tsvector;

-- Create function to update search_vector on insert/update
CREATE OR REPLACE FUNCTION requirements_search_vector_update() RETURNS trigger AS $$
BEGIN
    NEW.search_vector := 
        setweight(to_tsvector('english', COALESCE(NEW.reference_code, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.title, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(NEW.description, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(NEW.justification, '')), 'C');
    RETURN NEW;
END
$$ LANGUAGE plpgsql;

-- Create trigger to automatically update search_vector
CREATE TRIGGER requirements_search_vector_trigger
    BEFORE INSERT OR UPDATE OF title, description, justification, reference_code
    ON requirements
    FOR EACH ROW
    EXECUTE FUNCTION requirements_search_vector_update();

-- GIN index for fast full-text search
CREATE INDEX idx_requirements_search_vector 
    ON requirements 
    USING gin(search_vector);

-- Backfill search_vector for existing requirements
UPDATE requirements SET 
    search_vector = 
        setweight(to_tsvector('english', COALESCE(reference_code, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(title, '')), 'A') ||
        setweight(to_tsvector('english', COALESCE(description, '')), 'B') ||
        setweight(to_tsvector('english', COALESCE(justification, '')), 'C');

-- Table for tracking embedding indexing jobs (for async processing)
CREATE TABLE embedding_index_queue (
    id SERIAL PRIMARY KEY,
    requirement_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    project_id INTEGER NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',  -- pending, processing, completed, failed
    error_message TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    processed_at TIMESTAMP,
    UNIQUE(requirement_id)  -- Only one pending job per requirement
);

CREATE INDEX idx_embedding_index_queue_status 
    ON embedding_index_queue(status, created_at);

CREATE INDEX idx_embedding_index_queue_project 
    ON embedding_index_queue(project_id);
