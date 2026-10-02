-- Project deletion (issue #349).
--
-- 1. Baselines and locked saved views stay immutable, except while a project
--    is being purged: the repository sets the transaction-local setting
--    `marreq.purging_project` (set_config(..., true)) and deletes everything
--    of that project in one transaction. Only DELETE is let through; UPDATE
--    is still refused, and the setting disappears when the transaction ends.
-- 2. The audit log outlives the project: logs.project_id becomes SET NULL.

CREATE OR REPLACE FUNCTION forbid_baseline_update_delete() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' AND coalesce(current_setting('marreq.purging_project', true), '') <> '' THEN
        RETURN OLD;
    END IF;
    RAISE EXCEPTION 'Baselines are immutable: UPDATE and DELETE are not allowed';
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION forbid_locked_saved_view_mutate() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF OLD.locked AND coalesce(current_setting('marreq.purging_project', true), '') = '' THEN
            RAISE EXCEPTION 'Saved views used in a baseline are immutable';
        END IF;
        RETURN OLD;
    END IF;
    IF OLD.locked THEN
        RAISE EXCEPTION 'Saved views used in a baseline are immutable';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DO $$
DECLARE fk text;
BEGIN
    SELECT conname INTO fk FROM pg_constraint
    WHERE conrelid = 'logs'::regclass AND contype = 'f'
      AND conkey = ARRAY[(SELECT attnum FROM pg_attribute WHERE attrelid = 'logs'::regclass AND attname = 'project_id')];
    IF fk IS NOT NULL THEN
        EXECUTE format('ALTER TABLE logs DROP CONSTRAINT %I', fk);
    END IF;
END $$;
ALTER TABLE logs
    ADD CONSTRAINT logs_project_id_fkey FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL;
