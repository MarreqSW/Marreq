-- =============================================================================
-- Migration: Add missing foreign-key constraints
-- =============================================================================
-- Several columns acted as logical foreign keys without a DB-enforced
-- constraint, allowing orphaned references and inconsistent data.
--
-- Columns addressed:
--   projects.owner_id                -> users(id)       ON DELETE SET NULL
--   requirement_versions.author_id   -> users(id)       ON DELETE RESTRICT
--   requirement_versions.reviewer_id -> users(id)       ON DELETE RESTRICT
--   requirement_versions.category_id -> categories(id)  ON DELETE RESTRICT
--   tests.parent_id                  -> tests(id)       ON DELETE SET NULL
--
-- Column-shape changes:
--   author_id   was NOT NULL DEFAULT 0  →  NOT NULL   (sentinel default removed)
--   reviewer_id was NOT NULL DEFAULT 0  →  NOT NULL   (sentinel default removed)
--   category_id was NOT NULL DEFAULT 1  →  NOT NULL   (sentinel default removed)
--
-- Data-cleanup pass: orphaned rows (referencing non-existent rows) are healed
-- before each constraint is added; the migration will fail early if no fall-back
-- target exists (which should not happen in a correctly seeded database).
-- =============================================================================

-- ---------------------------------------------------------------------------
-- 1. projects.owner_id -> users(id)  ON DELETE SET NULL
-- ---------------------------------------------------------------------------
-- Column is already nullable.  Null-out any references to deleted users.
UPDATE projects
SET    owner_id = NULL
WHERE  owner_id IS NOT NULL
  AND  owner_id NOT IN (SELECT id FROM users);

ALTER TABLE projects
    ADD CONSTRAINT projects_owner_id_fk
        FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE SET NULL;

COMMENT ON COLUMN projects.owner_id IS
    'FK → users(id) ON DELETE SET NULL; NULL means the owning account was removed.';

-- ---------------------------------------------------------------------------
-- 2. requirement_versions.author_id -> users(id)  ON DELETE RESTRICT
-- ---------------------------------------------------------------------------
-- Remove the DEFAULT 0 sentinel that masked missing FK enforcement.
-- Orphaned rows (author_id not present in users) are reassigned to the earliest
-- user record; this handles stale seed/test data only—production data should
-- always carry a valid author.
ALTER TABLE requirement_versions
    ALTER COLUMN author_id DROP DEFAULT;

UPDATE requirement_versions
SET    author_id = (SELECT id FROM users ORDER BY id LIMIT 1)
WHERE  author_id NOT IN (SELECT id FROM users);

ALTER TABLE requirement_versions
    ADD CONSTRAINT requirement_versions_author_id_fk
        FOREIGN KEY (author_id) REFERENCES users(id) ON DELETE RESTRICT;

COMMENT ON COLUMN requirement_versions.author_id IS
    'FK → users(id) ON DELETE RESTRICT; the authoring user must exist before deletion.';

-- ---------------------------------------------------------------------------
-- 3. requirement_versions.reviewer_id -> users(id)  ON DELETE RESTRICT
-- ---------------------------------------------------------------------------
ALTER TABLE requirement_versions
    ALTER COLUMN reviewer_id DROP DEFAULT;

UPDATE requirement_versions
SET    reviewer_id = (SELECT id FROM users ORDER BY id LIMIT 1)
WHERE  reviewer_id NOT IN (SELECT id FROM users);

ALTER TABLE requirement_versions
    ADD CONSTRAINT requirement_versions_reviewer_id_fk
        FOREIGN KEY (reviewer_id) REFERENCES users(id) ON DELETE RESTRICT;

COMMENT ON COLUMN requirement_versions.reviewer_id IS
    'FK → users(id) ON DELETE RESTRICT; the reviewer must exist before deletion.';

-- ---------------------------------------------------------------------------
-- 4. requirement_versions.category_id -> categories(id)  ON DELETE RESTRICT
-- ---------------------------------------------------------------------------
-- Remove the DEFAULT 1 sentinel.
ALTER TABLE requirement_versions
    ALTER COLUMN category_id DROP DEFAULT;

-- Fix rows whose category_id does not exist in the categories table at all,
-- or whose category belongs to a different project than the requirement.
-- Reassign to the first category in the correct project.
UPDATE requirement_versions rv
SET    category_id = (
           SELECT c.id
           FROM   categories c
           JOIN   requirements req ON req.id = rv.requirement_id
           WHERE  c.project_id = req.project_id
           ORDER  BY c.id
           LIMIT  1
       )
WHERE  NOT EXISTS (
           SELECT 1
           FROM   categories c
           JOIN   requirements req ON req.id = rv.requirement_id
           WHERE  c.id = rv.category_id
             AND  c.project_id = req.project_id
       );

ALTER TABLE requirement_versions
    ADD CONSTRAINT requirement_versions_category_id_fk
        FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE RESTRICT;

COMMENT ON COLUMN requirement_versions.category_id IS
    'FK → categories(id) ON DELETE RESTRICT; must be a category belonging to the same project.';

-- Cross-project consistency trigger: category must belong to the same project
-- as the requirement that owns this version.
CREATE OR REPLACE FUNCTION check_rv_category_project_consistency()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    cat_project_id INTEGER;
    req_project_id INTEGER;
BEGIN
    SELECT project_id INTO cat_project_id
    FROM   categories WHERE id = NEW.category_id;

    SELECT r.project_id INTO req_project_id
    FROM   requirements r WHERE r.id = NEW.requirement_id;

    IF cat_project_id IS NULL THEN
        RAISE EXCEPTION
            '[cross_project] category % does not exist', NEW.category_id;
    END IF;
    IF req_project_id IS NULL THEN
        RAISE EXCEPTION
            '[cross_project] requirement % does not exist', NEW.requirement_id;
    END IF;
    IF cat_project_id <> req_project_id THEN
        RAISE EXCEPTION
            '[cross_project] category % belongs to project % but requirement version belongs to project %',
            NEW.category_id, cat_project_id, req_project_id;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER rv_category_project_consistency
    BEFORE INSERT OR UPDATE ON requirement_versions
    FOR EACH ROW EXECUTE FUNCTION check_rv_category_project_consistency();

-- ---------------------------------------------------------------------------
-- 5. tests.parent_id -> tests(id)  ON DELETE SET NULL
-- ---------------------------------------------------------------------------
-- Column is already nullable.  Null-out any dangling self-references.
UPDATE tests
SET    parent_id = NULL
WHERE  parent_id IS NOT NULL
  AND  parent_id NOT IN (SELECT id FROM tests);

ALTER TABLE tests
    ADD CONSTRAINT tests_parent_id_fk
        FOREIGN KEY (parent_id) REFERENCES tests(id) ON DELETE SET NULL;

COMMENT ON COLUMN tests.parent_id IS
    'Self-referencing FK → tests(id) ON DELETE SET NULL; NULL when parent test is deleted.';
