ALTER TABLE baseline_verifications DROP COLUMN IF EXISTS author_id;
ALTER TABLE baseline_verifications DROP COLUMN IF EXISTS reviewer_id;

ALTER TABLE verifications DROP COLUMN IF EXISTS status_set_at;
ALTER TABLE verifications DROP COLUMN IF EXISTS status_set_by;
ALTER TABLE verifications DROP COLUMN IF EXISTS reviewer_id;
ALTER TABLE verifications DROP COLUMN IF EXISTS author_id;

ALTER TABLE requirement_versions DROP COLUMN IF EXISTS reviewed_at;
ALTER TABLE requirement_versions DROP COLUMN IF EXISTS reviewed_by;

DROP TABLE IF EXISTS project_reviewers;
