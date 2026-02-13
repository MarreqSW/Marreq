-- =============================================================================
-- Requirement comments (immutable; attached to requirement or specific version)
-- =============================================================================
-- Comments are immutable after creation. Optional requirement_version_id tags
-- the comment to a specific version; NULL means comment is on the requirement generally.
-- =============================================================================

CREATE TABLE requirement_comments (
    id SERIAL PRIMARY KEY,
    requirement_id INTEGER NOT NULL REFERENCES requirements(id) ON DELETE CASCADE,
    requirement_version_id INTEGER REFERENCES requirement_versions(id) ON DELETE SET NULL,
    author_id INTEGER NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_requirement_comments_requirement ON requirement_comments(requirement_id);
CREATE INDEX idx_requirement_comments_version ON requirement_comments(requirement_version_id);
