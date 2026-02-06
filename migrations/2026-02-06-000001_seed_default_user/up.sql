-- Seed default user when no users exist (e.g. fresh Docker DB).
-- Login: alice / password
INSERT INTO users (username, name, email, is_admin, password_hash)
SELECT 'alice', 'Alice Johnson', 'alice@reqman.com', true, '$2b$12$XA9O8krsitwulDQm1Cx3rupcIVug8lckConqWLmBsn6kXKNApQE7m'
WHERE NOT EXISTS (SELECT 1 FROM users LIMIT 1);
