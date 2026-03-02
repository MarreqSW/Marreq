-- =============================================================================
-- Migration: enforce unique user identity (case-insensitive)
-- =============================================================================
-- Adds functional unique indexes on lower(username) and lower(email) so that
-- "Alice" and "alice" are treated as the same identity.
--
-- PREFLIGHT: the DO block below aborts the migration with a clear diagnostic
-- if case-insensitive duplicates already exist in the users table.  Clean them
-- up manually before re-running.
-- =============================================================================

DO $$
DECLARE
    dup_user_count  INTEGER;
    dup_email_count INTEGER;
    dup_report      TEXT;
BEGIN
    -- Count distinct case-insensitive username groups that have duplicates
    SELECT COUNT(*) INTO dup_user_count
    FROM (
        SELECT lower(username)
        FROM users
        GROUP BY lower(username)
        HAVING COUNT(*) > 1
    ) t;

    -- Count distinct case-insensitive email groups that have duplicates
    SELECT COUNT(*) INTO dup_email_count
    FROM (
        SELECT lower(email)
        FROM users
        GROUP BY lower(email)
        HAVING COUNT(*) > 1
    ) t;

    IF dup_user_count > 0 OR dup_email_count > 0 THEN
        -- Build a human-readable report of the conflicting values
        SELECT string_agg(line, E'\n') INTO dup_report
        FROM (
            SELECT '  [username] ' || lower(username) ||
                   ' — ' || COUNT(*) || ' rows (ids: ' ||
                   string_agg(id::text, ', ' ORDER BY id) || ')' AS line
            FROM users
            GROUP BY lower(username)
            HAVING COUNT(*) > 1

            UNION ALL

            SELECT '  [email]    ' || lower(email) ||
                   ' — ' || COUNT(*) || ' rows (ids: ' ||
                   string_agg(id::text, ', ' ORDER BY id) || ')' AS line
            FROM users
            GROUP BY lower(email)
            HAVING COUNT(*) > 1
        ) problems;

        RAISE EXCEPTION
            E'Cannot enforce unique user identity: % duplicate group(s) detected.\n'
            'Resolve the following conflicts before re-running this migration:\n%',
            dup_user_count + dup_email_count,
            dup_report;
    END IF;
END;
$$;

-- Replace the non-unique plain index with a case-insensitive unique index.
-- The functional index on lower(username) makes the plain one redundant.
DROP INDEX IF EXISTS idx_users_username;

CREATE UNIQUE INDEX idx_users_username_lower ON users (lower(username));
CREATE UNIQUE INDEX idx_users_email_lower    ON users (lower(email));
