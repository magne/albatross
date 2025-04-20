-- Add user_api_keys table for storing API key details

CREATE TABLE IF NOT EXISTS user_api_keys (
    key_id VARCHAR(255) PRIMARY KEY, -- Unique identifier for the key itself (e.g., key_abc123)
    user_id VARCHAR(36) NOT NULL REFERENCES users(user_id), -- Foreign key to the user (matches users.user_id type)
    tenant_id VARCHAR(36), -- Optional tenant association, denormalized for faster auth checks (matches tenants.tenant_id type)
    key_name VARCHAR(255) NOT NULL, -- User-provided name for the key
    api_key_hash VARCHAR(255) NOT NULL, -- Securely hashed API key (Argon2, bcrypt etc.)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ, -- Timestamp when the key was revoked, NULL if active
    last_used_at TIMESTAMPTZ -- Optional: Track last usage time
);

-- Index for efficient lookup during authentication (using user_id)
CREATE INDEX IF NOT EXISTS idx_user_api_keys_user_id ON user_api_keys(user_id);
-- An index on key_id is implicitly created by PRIMARY KEY constraint.
