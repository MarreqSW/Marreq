-- Ensure the seeded admin user "alice" can log in with password ChangeMe123!
-- (Argon2id hash). Idempotent: safe to run after every migration run or DB rebuild.
UPDATE users
SET password_hash = '$argon2id$v=19$m=19456,t=2,p=1$3o6cC/67ksnBxHCCF9rGHA$oWCATKyiKRCdDgWucvrMHinlWvzZNhqoUUvnpyCgOW0'
WHERE LOWER(username) = 'alice';
