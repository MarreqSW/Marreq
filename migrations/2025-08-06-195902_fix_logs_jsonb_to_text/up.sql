-- Change JSONB columns to TEXT in logs table
ALTER TABLE logs ALTER COLUMN old_values TYPE TEXT;
ALTER TABLE logs ALTER COLUMN new_values TYPE TEXT;
