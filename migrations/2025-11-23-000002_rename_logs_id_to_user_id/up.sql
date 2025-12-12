-- Rename the logs column that references users so it clearly denotes a user id.

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'logs' AND column_name = 'id'
    ) THEN
        EXECUTE 'ALTER TABLE logs RENAME COLUMN id TO user_id';
    END IF;
END;
$$;
