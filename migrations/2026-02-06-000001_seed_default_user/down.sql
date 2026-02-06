-- Remove seeded default user (only if present)
DELETE FROM users WHERE username = 'alice';
