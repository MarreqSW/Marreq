DROP TABLE IF EXISTS user_identities;

-- Fails safely while external-only users remain; assign a credential first.
ALTER TABLE users ALTER COLUMN password_hash SET NOT NULL;
