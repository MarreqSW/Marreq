ALTER TABLE baselines
    DROP COLUMN IF EXISTS source_view_definition,
    DROP COLUMN IF EXISTS source_saved_view_id;

DROP TRIGGER IF EXISTS saved_views_locked_immutable ON saved_views;
DROP FUNCTION IF EXISTS forbid_locked_saved_view_mutate() CASCADE;
DROP TABLE IF EXISTS saved_views;
