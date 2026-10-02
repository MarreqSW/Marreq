ALTER TABLE logs DROP CONSTRAINT IF EXISTS logs_project_id_fkey;
ALTER TABLE logs
    ADD CONSTRAINT logs_project_id_fkey FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE;

CREATE OR REPLACE FUNCTION forbid_locked_saved_view_mutate() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        IF OLD.locked THEN
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

CREATE OR REPLACE FUNCTION forbid_baseline_update_delete() RETURNS trigger AS $$
BEGIN
    RAISE EXCEPTION 'Baselines are immutable: UPDATE and DELETE are not allowed';
END;
$$ LANGUAGE plpgsql;
