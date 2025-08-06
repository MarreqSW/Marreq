-- Remove is_admin column from users table if it exists
DO $$ 
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns 
               WHERE table_name = 'users' AND column_name = 'is_admin') THEN
        ALTER TABLE users DROP COLUMN is_admin;
    END IF;
END $$;
