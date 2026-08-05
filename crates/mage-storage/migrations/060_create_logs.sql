CREATE TABLE IF NOT EXISTS logs (
    id              UUID        PRIMARY KEY,
    trace_id        UUID        NOT NULL,
    tenant_id       UUID        NOT NULL,
    task_id         UUID,
    level           TEXT        NOT NULL,
    source          TEXT        NOT NULL,
    message         TEXT        NOT NULL,
    fields          JSONB       NOT NULL DEFAULT '{}',
    created_by_id   UUID        NOT NULL REFERENCES actors(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    FOREIGN KEY (tenant_id, created_by_id)
        REFERENCES actors (tenant_id, id)
        ON DELETE CASCADE,

    FOREIGN KEY (tenant_id, task_id)
        REFERENCES tasks (tenant_id, id)
        ON DELETE CASCADE
);

CREATE INDEX idx_logs_trace_id
ON logs (trace_id, id DESC);

CREATE INDEX idx_logs_task_id
ON logs (task_id, id DESC)
WHERE task_id IS NOT NULL;

CREATE INDEX idx_logs_tenant_level_id
ON logs (tenant_id, level, id DESC);

CREATE INDEX idx_logs_tenant_id
ON logs (tenant_id, id DESC);
