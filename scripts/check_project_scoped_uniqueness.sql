-- =============================================================================
-- Pre-migration check: project-scoped uniqueness
-- =============================================================================
-- Run before applying migration 2026-03-03-000004_project_scoped_uniqueness
-- if the database already has data. Any non-empty result indicates duplicates
-- that must be resolved before the migration will succeed.
--
-- Usage: psql "$DATABASE_URL" -f scripts/check_project_scoped_uniqueness.sql
-- =============================================================================

\echo 'Duplicate (project_id, tag) in requirement_status:'
SELECT project_id, tag, count(*) AS n
FROM requirement_status
GROUP BY project_id, tag
HAVING count(*) > 1;

\echo 'Duplicate (project_id, tag) in test_status:'
SELECT project_id, tag, count(*) AS n
FROM test_status
GROUP BY project_id, tag
HAVING count(*) > 1;

\echo 'Duplicate (project_id, tag) in categories:'
SELECT project_id, tag, count(*) AS n
FROM categories
GROUP BY project_id, tag
HAVING count(*) > 1;

\echo 'Duplicate (project_id, tag) in applicability:'
SELECT project_id, tag, count(*) AS n
FROM applicability
GROUP BY project_id, tag
HAVING count(*) > 1;

\echo 'Duplicate (project_id, tag) in verification:'
SELECT project_id, tag, count(*) AS n
FROM verification
GROUP BY project_id, tag
HAVING count(*) > 1;
