-- Restore the original column name on logs.

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'logs' AND column_name = 'user_id'
    ) THEN
        EXECUTE 'ALTER TABLE logs RENAME COLUMN user_id TO id';
    END IF;
END;
$$;
