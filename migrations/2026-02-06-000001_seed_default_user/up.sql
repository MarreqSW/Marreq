-- Seed default user when no users exist (e.g. fresh Docker DB).
-- Login: alice / ChangeMe123!
INSERT INTO users (username, name, email, is_admin, password_hash)
SELECT 'alice', 'Alice Johnson', 'alice@reqman.com', true, '$argon2id$v=19$m=19456,t=2,p=1$3o6cC/67ksnBxHCCF9rGHA$oWCATKyiKRCdDgWucvrMHinlWvzZNhqoUUvnpyCgOW0'
WHERE NOT EXISTS (SELECT 1 FROM users LIMIT 1);
