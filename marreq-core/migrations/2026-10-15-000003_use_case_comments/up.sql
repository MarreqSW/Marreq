-- Comments on use cases (issue #401, epic #388). Immutable, like requirement
-- comments. A comment belongs to the use case, optionally to one version, and
-- optionally to one element (step or condition id) of its document.

CREATE TABLE use_case_comments (
    id SERIAL PRIMARY KEY,
    use_case_id INTEGER NOT NULL REFERENCES use_cases(id) ON DELETE CASCADE,
    use_case_version_id INTEGER REFERENCES use_case_versions(id) ON DELETE SET NULL,
    element_id VARCHAR(40),
    author_id INTEGER NOT NULL REFERENCES users(id),
    body TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);
CREATE INDEX idx_use_case_comments_use_case ON use_case_comments (use_case_id);
