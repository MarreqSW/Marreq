-- Snapshot of verifications at baseline creation time (denormalized copy).
-- No FK to verifications(id) so the snapshot row remains if a verification is deleted.
CREATE TABLE baseline_verifications (
    baseline_id          INTEGER NOT NULL REFERENCES baselines(id) ON DELETE CASCADE,
    verification_id       INTEGER NOT NULL,
    name                  VARCHAR NOT NULL,
    reference_code        VARCHAR NOT NULL,
    description           VARCHAR NOT NULL DEFAULT ' ',
    source                VARCHAR NOT NULL DEFAULT ' ',
    status_id             INTEGER NOT NULL,
    parent_id             INTEGER NULL,
    project_id            INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    verification_method_id INTEGER NULL,
    PRIMARY KEY (baseline_id, verification_id)
);
