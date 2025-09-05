-- Create events table for Event Sourcing
CREATE TABLE events (
    id SERIAL PRIMARY KEY,
    aggregate_id VARCHAR(36) NOT NULL,
    sequence BIGINT NOT NULL,
    event_type VARCHAR(255) NOT NULL,
    payload BYTEA NOT NULL,
    tenant_id VARCHAR(36), -- Optional tenant association for multi-tenancy
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (aggregate_id, sequence)
);

-- Indexes for performance
CREATE INDEX idx_events_aggregate_id ON events(aggregate_id);
CREATE INDEX idx_events_sequence ON events(aggregate_id, sequence);
CREATE INDEX idx_events_event_type ON events(event_type);
CREATE INDEX idx_events_tenant_id ON events(tenant_id);
CREATE INDEX idx_events_timestamp ON events(timestamp);
