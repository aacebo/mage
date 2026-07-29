CREATE TABLE IF NOT EXISTS chats (
    id              UUID        PRIMARY KEY,
    tenant_id       UUID        NOT NULL,
    name            TEXT,
    created_by_id   UUID        NOT NULL REFERENCES actors(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at       TIMESTAMPTZ,

    UNIQUE (id, tenant_id),
    FOREIGN KEY (tenant_id, created_by_id)
        REFERENCES actors (tenant_id, id)
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS chat_actors (
    chat_id         UUID        NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    actor_id        UUID        NOT NULL REFERENCES actors(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (chat_id, actor_id)
);

CREATE INDEX IF NOT EXISTS idx_chats_tenant_id
ON chats (tenant_id, id DESC);

CREATE INDEX IF NOT EXISTS idx_chats_created_by_id
ON chats (created_by_id, id DESC);
