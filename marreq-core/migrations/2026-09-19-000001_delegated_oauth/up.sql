CREATE TABLE oauth_clients (
    client_id VARCHAR(128) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    redirect_uris JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT oauth_clients_name_not_empty CHECK (name <> ''),
    CONSTRAINT oauth_clients_redirect_uris_array CHECK (jsonb_typeof(redirect_uris) = 'array')
);

CREATE TABLE oauth_grants (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    client_id VARCHAR(128) NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    scopes TEXT[] NOT NULL,
    resource VARCHAR(2048) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NULL,
    revoked_at TIMESTAMP NULL,
    CONSTRAINT oauth_grants_user_client_resource_key UNIQUE (user_id, client_id, resource)
);

CREATE TABLE oauth_authorization_codes (
    code_hash VARCHAR(64) PRIMARY KEY,
    grant_id INTEGER NOT NULL REFERENCES oauth_grants(id) ON DELETE CASCADE,
    client_id VARCHAR(128) NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    redirect_uri VARCHAR(2048) NOT NULL,
    code_challenge VARCHAR(128) NOT NULL,
    scopes TEXT[] NOT NULL,
    resource VARCHAR(2048) NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    used_at TIMESTAMP NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE oauth_access_tokens (
    token_hash VARCHAR(64) PRIMARY KEY,
    grant_id INTEGER NOT NULL REFERENCES oauth_grants(id) ON DELETE CASCADE,
    client_id VARCHAR(128) NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    scopes TEXT[] NOT NULL,
    resource VARCHAR(2048) NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_used_at TIMESTAMP NULL
);

CREATE TABLE oauth_refresh_tokens (
    token_hash VARCHAR(64) PRIMARY KEY,
    family_id VARCHAR(64) NOT NULL,
    grant_id INTEGER NOT NULL REFERENCES oauth_grants(id) ON DELETE CASCADE,
    client_id VARCHAR(128) NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    scopes TEXT[] NOT NULL,
    resource VARCHAR(2048) NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    used_at TIMESTAMP NULL,
    revoked_at TIMESTAMP NULL,
    replaced_by_hash VARCHAR(64) NULL
);

CREATE INDEX oauth_grants_user_id_idx ON oauth_grants(user_id);
CREATE INDEX oauth_grants_client_id_idx ON oauth_grants(client_id);
CREATE INDEX oauth_authorization_codes_expiry_idx ON oauth_authorization_codes(expires_at);
CREATE INDEX oauth_access_tokens_expiry_idx ON oauth_access_tokens(expires_at);
CREATE INDEX oauth_access_tokens_grant_idx ON oauth_access_tokens(grant_id);
CREATE INDEX oauth_refresh_tokens_family_idx ON oauth_refresh_tokens(family_id);
CREATE INDEX oauth_refresh_tokens_grant_idx ON oauth_refresh_tokens(grant_id);
