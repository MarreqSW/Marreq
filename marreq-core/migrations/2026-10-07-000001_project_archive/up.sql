-- Archived projects (issue #381): read-only for everyone and hidden from the
-- everyday project lists, until unarchived. Kept apart from `status`, which
-- any project Admin can change.
ALTER TABLE projects
    ADD COLUMN archived_at TIMESTAMP,
    ADD COLUMN archived_by INTEGER REFERENCES users(id) ON DELETE SET NULL;
