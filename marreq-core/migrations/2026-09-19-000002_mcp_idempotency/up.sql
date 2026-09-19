CREATE TABLE mcp_idempotency (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    principal_key VARCHAR(200) NOT NULL,
    target_key VARCHAR(200) NOT NULL,
    operation VARCHAR(100) NOT NULL,
    idempotency_key VARCHAR(200) NOT NULL,
    request_hash CHAR(64) NOT NULL,
    response_json JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL,
    PRIMARY KEY (user_id, principal_key, target_key, operation, idempotency_key)
);
CREATE INDEX mcp_idempotency_expires_idx ON mcp_idempotency(expires_at);
