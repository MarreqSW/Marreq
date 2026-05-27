-- =============================================================================
-- Bulk seed: tests (verifications) + matrix links for REQ-GEN-* requirements,
-- plus requirement version hierarchy (REFINES / RELATES_TO / DEPENDS_ON).
-- =============================================================================
-- Depends on: seed_space_project_bulk_requirements.sql (REQ-GEN-0001 … 0120).
-- Safe to re-run: skips existing reference_codes / matrix pairs / duplicate links.
-- Verification status: pseudorandom across Space Project verification_status rows (hashtext).
-- parent_id tree aligns with requirement REFINES seed: 0001 root; 0002–0060 → 0001; 0061–0085 → 0060;
-- 0086–0120 → 0001 (mission umbrella).
--
-- Usage:
--   psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f marreq-core/scripts/seed_space_project_bulk_tests_and_hierarchy.sql
-- =============================================================================

BEGIN;

WITH space AS (
    SELECT id AS project_id FROM projects WHERE slug = 'space-project' LIMIT 1
),
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
ver_status_pool AS (
    SELECT ARRAY(
        SELECT vs.id
        FROM verification_status vs
        INNER JOIN space s ON vs.project_id = s.project_id
        ORDER BY vs.tag
    ) AS ids
),
vm_test AS (
    SELECT vm.id AS verification_method_id
    FROM verification_methods vm
    JOIN space s ON vm.project_id = s.project_id
    WHERE vm.tag = 'TEST'
    LIMIT 1
),
ins_ver AS (
    INSERT INTO verifications (
        reference_code,
        name,
        description,
        status_id,
        source,
        project_id,
        author_id,
        reviewer_id,
        verification_method_id,
        parent_id
    )
    SELECT
        'TEST-GEN-' || lpad(g::text, 4, '0'),
        'Automated verification for ' || 'REQ-GEN-' || lpad(g::text, 4, '0'),
        'Bulk seed test case verifying requirement REQ-GEN-' || lpad(g::text, 4, '0')
            || ' (automated bench / regression).',
        (
            SELECT vp.ids[
                1 + abs(hashtext(('TEST-GEN-' || lpad(g::text, 4, '0'))::text || 'vs'))
                % cardinality(vp.ids)
            ]
            FROM ver_status_pool vp
        ),
        'Automated test bench — bulk seed',
        s.project_id,
        (SELECT p.ids[1 + (g - 1) % cardinality(p.ids)] FROM author_pool p),
        (SELECT p.ids[1 + (g + 2) % cardinality(p.ids)] FROM author_pool p),
        (SELECT verification_method_id FROM vm_test),
        NULL::integer
    FROM space s
    CROSS JOIN generate_series(1, 120) AS g
    WHERE NOT EXISTS (
        SELECT 1
        FROM verifications v
        WHERE v.project_id = s.project_id
          AND v.reference_code = 'TEST-GEN-' || lpad(g::text, 4, '0')
    )
    RETURNING id
)
SELECT COUNT(*) AS inserted_verifications FROM ins_ver;

-- Traceability: one verification per REQ-GEN requirement (same numeric suffix).
INSERT INTO matrix (req_id, verification_id, project_id, suspect)
SELECT
    r.id,
    v.id,
    r.project_id,
    false
FROM requirements r
JOIN verifications v
    ON v.project_id = r.project_id
   AND v.reference_code = replace(r.stable_code, 'REQ-', 'TEST-')
WHERE r.project_id = (SELECT id FROM projects WHERE slug = 'space-project' LIMIT 1)
  AND r.stable_code LIKE 'REQ-GEN-%'
ON CONFLICT (req_id, verification_id) DO NOTHING;

-- Requirement hierarchy: current versions only.
WITH space AS (
    SELECT id AS project_id FROM projects WHERE slug = 'space-project' LIMIT 1
),
rv AS (
    SELECT
        r.stable_code,
        rv.id AS version_id
    FROM requirements r
    JOIN requirement_versions rv ON rv.id = r.current_version_id
    JOIN space s ON r.project_id = s.project_id
    WHERE r.stable_code LIKE 'REQ-GEN-%'
),
root AS (
    SELECT version_id FROM rv WHERE stable_code = 'REQ-GEN-0001' LIMIT 1
),
parent_60 AS (
    SELECT version_id FROM rv WHERE stable_code = 'REQ-GEN-0060' LIMIT 1
),
-- REQ-GEN-0002 … 0060 refine mission-level REQ-GEN-0001
ins_refines_root AS (
    INSERT INTO requirement_version_links (
        source_version_id,
        target_version_id,
        link_type,
        rationale,
        project_id,
        created_at
    )
    SELECT
        c.version_id,
        (SELECT version_id FROM root),
        'REFINES',
        'Bulk seed: subsystem requirement refines mission-level parent REQ-GEN-0001.',
        (SELECT project_id FROM space),
        CURRENT_TIMESTAMP
    FROM generate_series(2, 60) AS g(gn)
    JOIN rv c ON c.stable_code = 'REQ-GEN-' || lpad(gn::text, 4, '0')
    WHERE NOT EXISTS (
        SELECT 1
        FROM requirement_version_links x
        WHERE x.source_version_id = c.version_id
          AND x.target_version_id = (SELECT version_id FROM root)
          AND x.link_type = 'REFINES'
    )
    RETURNING id
)
SELECT COUNT(*) AS inserted_refines_root FROM ins_refines_root;

WITH space AS (
    SELECT id AS project_id FROM projects WHERE slug = 'space-project' LIMIT 1
),
rv AS (
    SELECT
        r.stable_code,
        rv.id AS version_id
    FROM requirements r
    JOIN requirement_versions rv ON rv.id = r.current_version_id
    JOIN space s ON r.project_id = s.project_id
    WHERE r.stable_code LIKE 'REQ-GEN-%'
),
parent_60 AS (
    SELECT version_id FROM rv WHERE stable_code = 'REQ-GEN-0060' LIMIT 1
),
ins_refines_60 AS (
    INSERT INTO requirement_version_links (
        source_version_id,
        target_version_id,
        link_type,
        rationale,
        project_id,
        created_at
    )
    SELECT
        c.version_id,
        (SELECT version_id FROM parent_60),
        'REFINES',
        'Bulk seed: lower-level requirement refines REQ-GEN-0060.',
        (SELECT project_id FROM space),
        CURRENT_TIMESTAMP
    FROM generate_series(61, 85) AS g(gn)
    JOIN rv c ON c.stable_code = 'REQ-GEN-' || lpad(gn::text, 4, '0')
    WHERE NOT EXISTS (
        SELECT 1
        FROM requirement_version_links x
        WHERE x.source_version_id = c.version_id
          AND x.target_version_id = (SELECT version_id FROM parent_60)
          AND x.link_type = 'REFINES'
    )
    RETURNING id
)
SELECT COUNT(*) AS inserted_refines_tier2 FROM ins_refines_60;

-- Peer / dependency edges (same project; avoids cycles with REFINES tree above).
WITH space AS (
    SELECT id AS project_id FROM projects WHERE slug = 'space-project' LIMIT 1
),
rv AS (
    SELECT r.stable_code, rv.id AS version_id
    FROM requirements r
    JOIN requirement_versions rv ON rv.id = r.current_version_id
    JOIN space s ON r.project_id = s.project_id
    WHERE r.stable_code LIKE 'REQ-GEN-%'
),
pairs AS (
    SELECT * FROM (VALUES
        ('REQ-GEN-0090', 'REQ-GEN-0091', 'RELATES_TO', 'Bulk seed: sibling subsystem coupling.'),
        ('REQ-GEN-0092', 'REQ-GEN-0093', 'RELATES_TO', 'Bulk seed: interface handshake.'),
        ('REQ-GEN-0094', 'REQ-GEN-0096', 'DEPENDS_ON', 'Bulk seed: scheduling depends on power budget.'),
        ('REQ-GEN-0097', 'REQ-GEN-0099', 'DEPENDS_ON', 'Bulk seed: comm payload depends on thermal margin.')
    ) AS t(a_code, b_code, ltype, rat)
)
INSERT INTO requirement_version_links (
    source_version_id,
    target_version_id,
    link_type,
    rationale,
    project_id,
    created_at
)
SELECT
    va.version_id,
    vb.version_id,
    p.ltype,
    p.rat,
    (SELECT project_id FROM space),
    CURRENT_TIMESTAMP
FROM pairs p
JOIN rv va ON va.stable_code = p.a_code
JOIN rv vb ON vb.stable_code = p.b_code
WHERE NOT EXISTS (
    SELECT 1
    FROM requirement_version_links x
    WHERE x.source_version_id = va.version_id
      AND x.target_version_id = vb.version_id
      AND x.link_type = p.ltype
);

-- Authors, reviewers, randomized status, parent chain (mirrors REQ-GEN REFINES tiers:
-- 0001 root; 0002–0060 → parent 0001; 0061–0085 → parent 0060; 0086–0120 → root 0001)
UPDATE verifications v
SET
    author_id = ap.ids[1 + (split_part(v.reference_code, '-', 3)::int - 1) % cardinality(ap.ids)],
    reviewer_id = ap.ids[1 + (split_part(v.reference_code, '-', 3)::int + 2) % cardinality(ap.ids)],
    status_id = st.ids[1 + abs(hashtext(v.reference_code::text || 'vs')) % cardinality(st.ids)],
    parent_id = CASE
        WHEN split_part(v.reference_code, '-', 3)::int = 1 THEN NULL
        WHEN split_part(v.reference_code, '-', 3)::int BETWEEN 2 AND 60 THEN (
            SELECT x.id FROM verifications x
            WHERE x.project_id = v.project_id AND x.reference_code = 'TEST-GEN-0001'
        )
        WHEN split_part(v.reference_code, '-', 3)::int BETWEEN 61 AND 85 THEN (
            SELECT x.id FROM verifications x
            WHERE x.project_id = v.project_id AND x.reference_code = 'TEST-GEN-0060'
        )
        ELSE (
            SELECT x.id FROM verifications x
            WHERE x.project_id = v.project_id AND x.reference_code = 'TEST-GEN-0001'
        )
    END
FROM (
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
) AS ap,
(
    SELECT ARRAY(
        SELECT vs.id
        FROM verification_status vs
        JOIN projects p ON p.id = vs.project_id
        WHERE p.slug = 'space-project'
        ORDER BY vs.tag
    ) AS ids
) AS st
WHERE v.project_id = (SELECT id FROM projects WHERE slug = 'space-project' LIMIT 1)
  AND v.reference_code LIKE 'TEST-GEN-%';

COMMIT;

SELECT
    (SELECT COUNT(*) FROM verifications v
     JOIN projects p ON p.id = v.project_id
     WHERE p.slug = 'space-project' AND v.reference_code LIKE 'TEST-GEN-%') AS bulk_tests,
    (SELECT COUNT(*) FROM matrix m
     JOIN projects p ON p.id = m.project_id
     JOIN requirements r ON r.id = m.req_id
     WHERE p.slug = 'space-project' AND r.stable_code LIKE 'REQ-GEN-%') AS matrix_links_bulk_reqs,
    (SELECT COUNT(*) FROM requirement_version_links rvl
     JOIN projects p ON p.id = rvl.project_id
     WHERE p.slug = 'space-project') AS total_version_links_in_project;
