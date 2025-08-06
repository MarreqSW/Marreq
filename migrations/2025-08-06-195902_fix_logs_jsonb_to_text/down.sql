-- Revert TEXT columns back to JSONB in logs table
ALTER TABLE logs ALTER COLUMN old_values TYPE JSONB;
ALTER TABLE logs ALTER COLUMN new_values TYPE JSONB;
