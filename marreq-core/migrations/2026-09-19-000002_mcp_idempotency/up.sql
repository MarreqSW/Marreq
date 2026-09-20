CREATE TABLE mcp_idempotency (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    principal_key VARCHAR(200) NOT NULL,
    target_key VARCHAR(200) NOT NULL,
    operation VARCHAR(100) NOT NULL,
    idempotency_key VARCHAR(200) NOT NULL,
    request_hash CHAR(64) NOT NULL,
    response_json JSONB,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    lease_expires_at TIMESTAMP NOT NULL,
    PRIMARY KEY (user_id, principal_key, target_key, operation, idempotency_key)
);

ALTER TABLE requirements ADD COLUMN mcp_idempotency_identity CHAR(64) UNIQUE;
ALTER TABLE verifications ADD COLUMN mcp_idempotency_identity CHAR(64) UNIQUE;
ALTER TABLE baselines ADD COLUMN mcp_idempotency_identity CHAR(64) UNIQUE;
ALTER TABLE requirement_comments ADD COLUMN mcp_idempotency_identity CHAR(64) UNIQUE;
