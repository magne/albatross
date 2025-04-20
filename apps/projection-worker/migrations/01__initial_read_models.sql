-- Initial Read Models for Tenants and Users

-- Tenants Table
CREATE TABLE tenants (
    tenant_id VARCHAR(36) PRIMARY KEY, -- Assuming UUID stored as string
    name VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
    -- Add other queryable tenant fields later
);

-- Users Table
CREATE TABLE users (
    user_id VARCHAR(36) PRIMARY KEY, -- Assuming UUID stored as string
    tenant_id VARCHAR(36) NULL, -- Nullable for Platform Admins
    username VARCHAR(100) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    role VARCHAR(50) NOT NULL, -- Store role as string (e.g., 'PlatformAdmin', 'Pilot')
    password_hash VARCHAR(255) NOT NULL, -- Store the password hash
    -- api_keys JSONB NULL, -- Store API key hashes (key_id -> hash) if needed for querying, or keep only in aggregate
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE SET NULL -- Or CASCADE? Decide later.
);

-- Indexes for common query patterns
CREATE INDEX idx_users_tenant_id ON users(tenant_id);
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);

-- Function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION trigger_set_timestamp()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Triggers to update updated_at on table updates
CREATE TRIGGER set_timestamp_tenants
BEFORE UPDATE ON tenants
FOR EACH ROW
EXECUTE FUNCTION trigger_set_timestamp();

CREATE TRIGGER set_timestamp_users
BEFORE UPDATE ON users
FOR EACH ROW
EXECUTE FUNCTION trigger_set_timestamp();
