-- File attachments on requirements and verifications (issue #241).
--
-- File contents live outside the database in a content-addressed blob store
-- (`<MARREQ_ATTACHMENTS_DIR>/ab/cd/<sha256>`); rows only reference the hash.
-- The entity reference is polymorphic, so services soft-delete the rows when
-- a requirement or verification is deleted.
CREATE TABLE attachments (
    id SERIAL PRIMARY KEY,
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    entity_type VARCHAR(20) NOT NULL
        CONSTRAINT attachments_entity_type_check CHECK (entity_type IN ('requirement', 'verification')),
    entity_id INTEGER NOT NULL,
    sha256 CHAR(64) NOT NULL
        CONSTRAINT attachments_sha256_check CHECK (sha256 ~ '^[0-9a-f]{64}$'),
    size_bytes BIGINT NOT NULL CONSTRAINT attachments_size_check CHECK (size_bytes > 0),
    original_filename VARCHAR(255) NOT NULL,
    content_type VARCHAR(100) NOT NULL,
    uploaded_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMP NULL
);

CREATE INDEX attachments_entity_live_idx
    ON attachments (project_id, entity_type, entity_id)
    WHERE deleted_at IS NULL;
CREATE INDEX attachments_sha256_idx ON attachments (sha256);

-- Per-project override of MARREQ_PROJECT_STORAGE_QUOTA_MB (instance admins only).
CREATE TABLE project_storage_quotas (
    project_id INTEGER PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    quota_bytes BIGINT NOT NULL CONSTRAINT project_storage_quotas_positive CHECK (quota_bytes > 0),
    updated_by INTEGER REFERENCES users(id) ON DELETE SET NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Attachments live at baseline time. Rows referenced here are soft-deleted
-- only, so the file stays downloadable from the baseline.
CREATE TABLE baseline_attachments (
    baseline_id INTEGER NOT NULL REFERENCES baselines(id) ON DELETE CASCADE,
    attachment_id INTEGER NOT NULL REFERENCES attachments(id) ON DELETE RESTRICT,
    PRIMARY KEY (baseline_id, attachment_id)
);
CREATE INDEX baseline_attachments_attachment_idx ON baseline_attachments (attachment_id);

CREATE TRIGGER baseline_attachments_immutable
    BEFORE UPDATE OR DELETE ON baseline_attachments
    FOR EACH ROW EXECUTE FUNCTION forbid_baseline_update_delete();
