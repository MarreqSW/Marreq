-- Saved report templates (issue #354): which sections a report document has,
-- in which order and with which options, plus its document fields. Private
-- templates are visible to their owner; shared ones to every project member.

CREATE TABLE report_templates (
    id          SERIAL PRIMARY KEY,
    project_id  INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    -- A template is a personal setting: it goes with its owner.
    owner_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name        VARCHAR(120) NOT NULL CHECK (btrim(name) <> ''),
    report_type VARCHAR(20) NOT NULL CHECK (report_type IN ('vcd', 'coverage')),
    visibility  VARCHAR(20) NOT NULL DEFAULT 'private'
                CHECK (visibility IN ('private', 'shared')),
    definition  JSONB NOT NULL CHECK (jsonb_typeof(definition) = 'object'),
    created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX idx_report_templates_project_owner_name
    ON report_templates (project_id, owner_id, lower(name));
CREATE INDEX idx_report_templates_project ON report_templates (project_id);
