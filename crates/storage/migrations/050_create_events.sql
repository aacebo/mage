CREATE TABLE IF NOT EXISTS events (
    id              UUID            PRIMARY KEY,
    trace_id        UUID            NOT NULL,
    tenant_id       UUID            NOT NULL,
    actor_id        UUID            REFERENCES actors(id) ON DELETE CASCADE,
    chat_id         UUID            REFERENCES chats(id) ON DELETE CASCADE,
    message_id      UUID            REFERENCES messages(id) ON DELETE CASCADE,
    task_id         UUID            REFERENCES tasks(id) ON DELETE CASCADE,
    key             TEXT            NOT NULL,
    data            JSONB           NOT NULL,
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_events_tenant_id
ON events (tenant_id, id DESC);

CREATE INDEX IF NOT EXISTS idx_events_tenant_key_id
ON events (tenant_id, key, id DESC);

CREATE INDEX IF NOT EXISTS idx_events_trace_id
ON events (trace_id, id DESC);

CREATE INDEX IF NOT EXISTS idx_events_actor_id
ON events (actor_id, id DESC)
WHERE actor_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_events_chat_id
ON events (chat_id, id DESC)
WHERE chat_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_events_message_id
ON events (message_id, id DESC)
WHERE message_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_events_task_id
ON events (task_id, id DESC)
WHERE task_id IS NOT NULL;
