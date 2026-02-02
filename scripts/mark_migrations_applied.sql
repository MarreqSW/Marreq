-- Backfill Diesel migration history so "diesel migration run" only runs the new migration.
-- Run this ONCE against your existing database when the tables already exist but
-- __diesel_schema_migrations is empty or out of sync.
--
-- Usage: psql "$DATABASE_URL" -f scripts/mark_migrations_applied.sql
--
-- Then run: diesel migration run

-- Ensure the migrations table exists (Diesel 2.x schema).
-- Use VARCHAR(100) so long migration names (e.g. 2026-02-02-221141_add_requirement_verification_methods) fit.
CREATE TABLE IF NOT EXISTS __diesel_schema_migrations (
    version VARCHAR(100) PRIMARY KEY NOT NULL
);

-- Widen version if table already existed with VARCHAR(50)
ALTER TABLE __diesel_schema_migrations ALTER COLUMN version TYPE VARCHAR(100);

-- Mark every migration before "add_requirement_verification_methods" as already applied.
-- Duplicate key errors are ignored (INSERT ... ON CONFLICT DO NOTHING).
INSERT INTO __diesel_schema_migrations (version) VALUES
  ('00000000000000_diesel_initial_setup'),
  ('2022-11-07-135533_requirements'),
  ('2025-08-03-090750_add_applicability'),
  ('2025-08-03-093841_add_justification_to_requirements'),
  ('2025-08-03-102340_add_password_to_users'),
  ('2025-08-06-094732_add_admin_to_users'),
  ('2025-08-06-154739_create_projects_table'),
  ('2025-08-06-160126_add_project_id_to_verification'),
  ('2025-08-06-190646_create_logs_table'),
  ('2025-08-06-195902_fix_logs_jsonb_to_text'),
  ('2025-08-09-191531_fix_req_17_title_reference'),
  ('2025-08-09-202426_fix_swapped_title_reference_columns'),
  ('2025-08-09-202526_fix_req_17_after_column_swap'),
  ('2025-08-09-203032_fix_empty_project_swapped_fields'),
  ('2025-08-09-210044_fix_req_prop_002_title_description'),
  ('2025-09-06-225009_add_test_reference'),
  ('2025-09-06-230423_split_status_tables'),
  ('2025-09-20-000000_add_project_members'),
  ('2025-11-02-000000_remove_req_link'),
  ('2025-11-23-000001_rename_status_id_to_test_status'),
  ('2025-11-23-000002_rename_logs_id_to_user_id'),
  ('2025-11-23-000003_create_project_status_table'),
  ('2025-11-23-000004_add_foreign_key_constraints'),
  ('2025-11-23-000005_add_check_constraints'),
  ('2025-11-23-000006_add_performance_indexes'),
  ('2025-11-23-000007_make_deadline_nullable'),
  ('2025-11-23-000008_add_unique_reference_code'),
  ('2025-11-23-000009_fix_primary_key_naming_consistency'),
  ('2025-12-09-000000_replace_project_status_with_enum'),
  ('2025-12-10-000000_make_project_status_not_null'),
  ('2026-01-29-000001_add_semantic_search'),
  ('2026-01-30-144348-0000_fix_embedding_dimensions')
ON CONFLICT (version) DO NOTHING;
