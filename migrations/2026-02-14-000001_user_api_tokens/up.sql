-- API tokens for headless auth (e.g. MCP server).
-- token_hash: SHA-256 hex of the raw token; raw token is never stored.
-- project_id: optional scope; when set, token only valid for that project.
CREATE TABLE user_api_tokens (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(64) NOT NULL,
    name VARCHAR(255),
    project_id INTEGER REFERENCES projects(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP,
    UNIQUE(token_hash)
);

CREATE INDEX idx_user_api_tokens_token_hash ON user_api_tokens(token_hash);
CREATE INDEX idx_user_api_tokens_user_id ON user_api_tokens(user_id);
