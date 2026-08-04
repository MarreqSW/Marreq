-- Saved views & queries (issue #110)
-- Project-scoped named views storing filter/sort/column definition as JSONB.

CREATE TABLE saved_views (
    id              SERIAL PRIMARY KEY,
    project_id      INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    owner_id        INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    name            VARCHAR(255) NOT NULL,
    description     TEXT,
    visibility      VARCHAR(20) NOT NULL DEFAULT 'private'
                    CHECK (visibility IN ('private', 'shared')),
    definition      JSONB NOT NULL,
    locked          BOOLEAN NOT NULL DEFAULT FALSE,
    locked_at       TIMESTAMP,
    created_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT saved_views_definition_is_object
        CHECK (jsonb_typeof(definition) = 'object')
);

CREATE UNIQUE INDEX idx_saved_views_project_owner_name
    ON saved_views (project_id, owner_id, lower(name));
CREATE INDEX idx_saved_views_project_visibility
    ON saved_views (project_id, visibility);
CREATE INDEX idx_saved_views_project_owner
    ON saved_views (project_id, owner_id);

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

CREATE TRIGGER saved_views_locked_immutable
    BEFORE UPDATE OR DELETE ON saved_views
    FOR EACH ROW EXECUTE FUNCTION forbid_locked_saved_view_mutate();

-- Baseline may record which view produced it (Phase C); nullable for existing rows.
ALTER TABLE baselines
    ADD COLUMN source_saved_view_id INTEGER
        REFERENCES saved_views(id) ON DELETE RESTRICT,
    ADD COLUMN source_view_definition JSONB;
