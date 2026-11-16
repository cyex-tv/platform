-- Phase 5: event WebSocket sessions (ADR 0005), so `resume` knows a connection's topics after a
-- reconnect, also to another instance or after a restart. Kept 24 h, the replay window.

CREATE TABLE event_sessions (
    id         uuid PRIMARY KEY,
    topics     text[] NOT NULL DEFAULT '{}' CHECK (cardinality(topics) <= 500),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX event_sessions_updated ON event_sessions (updated_at);
