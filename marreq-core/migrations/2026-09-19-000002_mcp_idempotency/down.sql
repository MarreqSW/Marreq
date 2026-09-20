ALTER TABLE requirement_comments DROP COLUMN IF EXISTS mcp_idempotency_identity;
ALTER TABLE baselines DROP COLUMN IF EXISTS mcp_idempotency_identity;
ALTER TABLE verifications DROP COLUMN IF EXISTS mcp_idempotency_identity;
ALTER TABLE requirements DROP COLUMN IF EXISTS mcp_idempotency_identity;
DROP TABLE IF EXISTS mcp_idempotency;
