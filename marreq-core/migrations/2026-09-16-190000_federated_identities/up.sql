ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;

CREATE TABLE user_identities (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider_key VARCHAR(64) NOT NULL,
    issuer VARCHAR(2048) NOT NULL,
    subject VARCHAR(1024) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_login_at TIMESTAMP NULL,
    CONSTRAINT user_identities_issuer_subject_key UNIQUE (issuer, subject),
    CONSTRAINT user_identities_provider_key_not_empty CHECK (provider_key <> ''),
    CONSTRAINT user_identities_issuer_not_empty CHECK (issuer <> ''),
    CONSTRAINT user_identities_subject_not_empty CHECK (subject <> '')
);

CREATE INDEX user_identities_user_id_idx ON user_identities(user_id);
