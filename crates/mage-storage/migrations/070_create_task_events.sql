CREATE TABLE IF NOT EXISTS task_events (
    id              UUID        PRIMARY KEY,
    tenant_id       UUID        NOT NULL,
    task_id         UUID        NOT NULL,
    sequence        BIGINT      NOT NULL,
    type            TEXT        NOT NULL,
    data            JSONB       NOT NULL DEFAULT '{}',
    created_by_id   UUID        NOT NULL REFERENCES actors(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (task_id, sequence),
    FOREIGN KEY (tenant_id, task_id)
        REFERENCES tasks (tenant_id, id)
        ON DELETE CASCADE,

    FOREIGN KEY (tenant_id, created_by_id)
        REFERENCES actors (tenant_id, id)
        ON DELETE CASCADE
);

CREATE INDEX idx_task_events_sequence
ON task_events (task_id, sequence);

CREATE INDEX idx_task_events_tenant_created
ON task_events (tenant_id, created_at DESC);
