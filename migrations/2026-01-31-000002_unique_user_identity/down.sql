-- Revert: remove unique identity indexes, restore original plain index
DROP INDEX IF EXISTS idx_users_username_lower;
DROP INDEX IF EXISTS idx_users_email_lower;

CREATE INDEX idx_users_username ON users (username);
