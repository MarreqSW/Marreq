-- Idempotent: when schema already exists (e.g. from initdb.d running init_complete.sql),
-- mark baseline/seed as applied so "diesel migration run" does not re-run them.
-- Only inserts when "projects" table exists (avoids marking applied on empty DB).
CREATE TABLE IF NOT EXISTS __diesel_schema_migrations (
    version VARCHAR(100) PRIMARY KEY NOT NULL
);

INSERT INTO __diesel_schema_migrations (version)
SELECT v.version FROM (VALUES
  ('2026-01-31-000001_baseline_schema'),
  ('2026-02-06-000001_seed_default_user')
) AS v(version)
WHERE EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'projects')
ON CONFLICT (version) DO NOTHING;
