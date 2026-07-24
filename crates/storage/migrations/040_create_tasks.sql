CREATE TABLE IF NOT EXISTS tasks (
    id              UUID            PRIMARY KEY,
    trace_id        UUID            NOT NULL,
    tenant_id       UUID            NOT NULL,
    chat_id         UUID            NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    message_id      UUID            REFERENCES messages(id) ON DELETE CASCADE,
    agent_id        UUID            REFERENCES actors(id) ON DELETE CASCADE,
    name            TEXT            NOT NULL,
    status          TEXT            NOT NULL,
    input           JSONB,
    output          JSONB,
    error           JSONB,
    attempts        INT             NOT NULL DEFAULT 0,
    max_attempts    INT             NOT NULL DEFAULT 3,
    started_at      TIMESTAMPTZ,
    ended_at        TIMESTAMPTZ,
    created_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ     NOT NULL DEFAULT NOW(),

    UNIQUE (id, tenant_id)
);

ALTER TABLE tasks
ADD COLUMN parent_id UUID REFERENCES tasks(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_tasks_tenant_id
ON tasks (tenant_id, id DESC);

CREATE INDEX IF NOT EXISTS idx_tasks_tenant_status_id
ON tasks (tenant_id, status, id DESC);

CREATE INDEX IF NOT EXISTS idx_tasks_trace_id
ON tasks (trace_id, id DESC);

CREATE INDEX IF NOT EXISTS idx_tasks_parent_id
ON tasks (parent_id, id DESC)
WHERE parent_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_tasks_chat_id
ON tasks (chat_id, id DESC);

CREATE INDEX IF NOT EXISTS idx_tasks_message_id
ON tasks (message_id, id DESC)
WHERE message_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_tasks_agent_id
ON tasks (agent_id, id DESC)
WHERE agent_id IS NOT NULL;
