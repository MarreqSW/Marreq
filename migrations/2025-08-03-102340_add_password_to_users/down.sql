-- Remove password column from users table
ALTER TABLE users DROP COLUMN password_hash;
