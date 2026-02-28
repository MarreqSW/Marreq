-- Idempotent: when schema already exists (e.g. from initdb.d running init_complete.sql),
-- mark baseline/seed as applied so "diesel migration run" does not re-run them.
-- Run this only against a DB that already has the full schema (e.g. from init_complete.sql).
CREATE TABLE IF NOT EXISTS __diesel_schema_migrations (
    version VARCHAR(100) PRIMARY KEY NOT NULL
);

INSERT INTO __diesel_schema_migrations (version)
VALUES
  ('2026-01-31-000001_baseline_schema'),
  ('2026-02-06-000001_seed_default_user')
ON CONFLICT (version) DO NOTHING;
