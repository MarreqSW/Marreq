-- Project-level reviewer pool: only these users may change requirement/verification status and version approval.
CREATE TABLE project_reviewers (
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (project_id, user_id),
    CONSTRAINT project_reviewers_member_fk
        FOREIGN KEY (project_id, user_id)
        REFERENCES project_members (project_id, user_id)
        ON DELETE CASCADE
);

-- Existing Admin + Reviewer roles become project reviewers (preserves prior ApproveVersions-style behavior).
INSERT INTO project_reviewers (project_id, user_id)
SELECT project_id, user_id FROM project_members WHERE role IN (1, 2)
ON CONFLICT DO NOTHING;

-- Who marked the version as reviewed (approved already has approved_by / approved_at).
ALTER TABLE requirement_versions
    ADD COLUMN reviewed_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    ADD COLUMN reviewed_at TIMESTAMP;

-- Verifications: assigned author/reviewer + last status change audit.
ALTER TABLE verifications
    ADD COLUMN author_id INTEGER REFERENCES users(id) ON DELETE RESTRICT,
    ADD COLUMN reviewer_id INTEGER REFERENCES users(id) ON DELETE RESTRICT,
    ADD COLUMN status_set_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    ADD COLUMN status_set_at TIMESTAMP;

-- Backfill author_id / reviewer_id: project owner, else smallest user id.
UPDATE verifications v
SET
    author_id = COALESCE(
        (SELECT p.owner_id FROM projects p WHERE p.id = v.project_id),
        (SELECT MIN(u.id) FROM users u)
    ),
    reviewer_id = COALESCE(
        (SELECT p.owner_id FROM projects p WHERE p.id = v.project_id),
        (SELECT MIN(u.id) FROM users u)
    )
WHERE author_id IS NULL OR reviewer_id IS NULL;

UPDATE verifications
SET author_id = (SELECT MIN(id) FROM users), reviewer_id = (SELECT MIN(id) FROM users)
WHERE author_id IS NULL OR reviewer_id IS NULL;

ALTER TABLE verifications
    ALTER COLUMN author_id SET NOT NULL,
    ALTER COLUMN reviewer_id SET NOT NULL;

-- Baseline snapshot of verifications: include author/reviewer.
ALTER TABLE baseline_verifications
    ADD COLUMN author_id INTEGER REFERENCES users(id) ON DELETE RESTRICT,
    ADD COLUMN reviewer_id INTEGER REFERENCES users(id) ON DELETE RESTRICT;

UPDATE baseline_verifications bv
SET
    author_id = v.author_id,
    reviewer_id = v.reviewer_id
FROM verifications v
WHERE v.id = bv.verification_id AND bv.author_id IS NULL;

UPDATE baseline_verifications
SET
    author_id = COALESCE(author_id, (SELECT MIN(id) FROM users)),
    reviewer_id = COALESCE(reviewer_id, (SELECT MIN(id) FROM users))
WHERE author_id IS NULL OR reviewer_id IS NULL;

ALTER TABLE baseline_verifications
    ALTER COLUMN author_id SET NOT NULL,
    ALTER COLUMN reviewer_id SET NOT NULL;
