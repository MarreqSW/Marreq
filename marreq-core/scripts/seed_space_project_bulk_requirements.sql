-- =============================================================================
-- Bulk seed: ≥100 requirements for Space Project (slug: space-project)
-- =============================================================================
-- Safe to re-run: skips stable_codes that already exist for this project.
-- Requirement status: pseudorandom across Space Project requirement_status rows (hashtext per stable_code).
--
-- Usage:
--   psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f marreq-core/scripts/seed_space_project_bulk_requirements.sql
-- Docker example:
--   docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d marreq -v ON_ERROR_STOP=1 < marreq-core/scripts/seed_space_project_bulk_requirements.sql
-- =============================================================================

BEGIN;

WITH space AS (
    SELECT id AS project_id FROM projects WHERE slug = 'space-project' LIMIT 1
),
-- Space/Marreq demo users (same set as init_complete.sql)
author_pool AS (
    SELECT ARRAY(
        SELECT u.id
        FROM users u
        WHERE u.username IN (
            'alice',
            'dr_smith',
            'eng_jones',
            'tech_lee',
            'qa_wilson',
            'sysadmin'
        )
        ORDER BY u.username
    ) AS ids
),
req_status_pool AS (
    SELECT ARRAY(
        SELECT rs.id
        FROM requirement_status rs
        INNER JOIN space s ON rs.project_id = s.project_id
        ORDER BY rs.tag
    ) AS ids
),
inserted_req AS (
    INSERT INTO requirements (project_id, stable_code)
    SELECT sp.project_id, 'REQ-GEN-' || lpad(g::text, 4, '0')
    FROM space sp
    CROSS JOIN generate_series(1, 120) AS g
    WHERE NOT EXISTS (
        SELECT 1
        FROM requirements r
        WHERE r.project_id = sp.project_id
          AND r.stable_code = 'REQ-GEN-' || lpad(g::text, 4, '0')
    )
    RETURNING id, project_id, stable_code
),
cat_tags AS (
    SELECT ARRAY['PWR', 'COMM', 'ACS', 'THERM', 'PAY', 'PROP', 'STRUCT', 'SW']::text[] AS tags
),
ins_versions AS (
    INSERT INTO requirement_versions (
        requirement_id,
        title,
        description,
        category_id,
        applicability_id,
        status_id,
        author_id,
        reviewer_id,
        created_at,
        deadline_date
    )
    SELECT
        ir.id,
        ir.stable_code,
        'Bulk-generated requirement: the satellite subsystem shall satisfy design constraints identified as '
            || ir.stable_code
            || ' for the Space mission (automated seed).',
        (
            SELECT c.id
            FROM categories c
            WHERE c.project_id = ir.project_id
              AND c.tag = (SELECT tags[1 + (abs(hashtext(ir.stable_code::text)) % 8)] FROM cat_tags)
            LIMIT 1
        ),
        (
            SELECT a.id
            FROM applicability a
            WHERE a.project_id = ir.project_id AND a.tag = 'ALL'
            LIMIT 1
        ),
        (
            SELECT sp.ids[1 + abs(hashtext(ir.stable_code::text || '::reqstat')) % cardinality(sp.ids)]
            FROM req_status_pool sp
        ),
        (
            SELECT p.ids[1 + (split_part(ir.stable_code, '-', 3)::int - 1) % cardinality(p.ids)]
            FROM author_pool p
        ),
        (
            SELECT p.ids[1 + (split_part(ir.stable_code, '-', 3)::int + 2) % cardinality(p.ids)]
            FROM author_pool p
        ),
        CURRENT_TIMESTAMP,
        CURRENT_TIMESTAMP + interval '180 days'
    FROM inserted_req ir
    RETURNING id AS version_id, requirement_id
)
INSERT INTO requirement_version_verification_methods (requirement_version_id, verification_method_id)
SELECT
    iv.version_id,
    COALESCE(
        (
            SELECT vm.id
            FROM verification_methods vm
            JOIN projects p ON p.id = vm.project_id
            WHERE p.slug = 'space-project'
              AND vm.tag = 'TEST'
            LIMIT 1
        ),
        (
            SELECT vm.id
            FROM verification_methods vm
            JOIN projects p ON p.id = vm.project_id
            WHERE p.slug = 'space-project'
            ORDER BY vm.id
            LIMIT 1
        )
    )
FROM ins_versions iv;

-- Point requirement containers to current version + first_created_at
UPDATE requirements r
SET
    current_version_id = (
        SELECT rv.id
        FROM requirement_versions rv
        WHERE rv.requirement_id = r.id
        ORDER BY rv.id DESC
        LIMIT 1
    ),
    first_created_at = COALESCE(
        r.first_created_at,
        (
            SELECT rv.created_at
            FROM requirement_versions rv
            WHERE rv.requirement_id = r.id
            ORDER BY rv.id ASC
            LIMIT 1
        )
    )
WHERE r.project_id = (SELECT id FROM projects WHERE slug = 'space-project' LIMIT 1)
  AND r.stable_code LIKE 'REQ-GEN-%'
  AND r.current_version_id IS NULL;

-- Rotate author/reviewer/status on all bulk rows (also refreshes already-seeded DBs)
WITH author_pool AS (
    SELECT ARRAY(
        SELECT u.id
        FROM users u
        WHERE u.username IN (
            'alice',
            'dr_smith',
            'eng_jones',
            'tech_lee',
            'qa_wilson',
            'sysadmin'
        )
        ORDER BY u.username
    ) AS ids
),
req_status_pool AS (
    SELECT ARRAY(
        SELECT rs.id
        FROM requirement_status rs
        JOIN projects p ON p.id = rs.project_id
        WHERE p.slug = 'space-project'
        ORDER BY rs.tag
    ) AS ids
)
UPDATE requirement_versions rv
SET
    author_id = (
        SELECT p.ids[1 + (split_part(r.stable_code, '-', 3)::int - 1) % cardinality(p.ids)]
        FROM author_pool p
    ),
    reviewer_id = (
        SELECT p.ids[1 + (split_part(r.stable_code, '-', 3)::int + 2) % cardinality(p.ids)]
        FROM author_pool p
    ),
    status_id = (
        SELECT sp.ids[1 + abs(hashtext(r.stable_code::text || '::reqstat')) % cardinality(sp.ids)]
        FROM req_status_pool sp
    )
FROM requirements r
WHERE r.id = rv.requirement_id
  AND r.project_id = (SELECT id FROM projects WHERE slug = 'space-project' LIMIT 1)
  AND r.stable_code LIKE 'REQ-GEN-%'
  AND rv.id = r.current_version_id;

COMMIT;

-- Summary (optional)
SELECT COUNT(*) AS bulk_requirements_in_space_project
FROM requirements r
JOIN projects p ON p.id = r.project_id
WHERE p.slug = 'space-project' AND r.stable_code LIKE 'REQ-GEN-%';
