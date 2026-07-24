CREATE TABLE IF NOT EXISTS actors (
    id              UUID        PRIMARY KEY,
    tenant_id       UUID        NOT NULL,
    external_id     TEXT,
    role            TEXT        NOT NULL, -- user, agent
    name            TEXT        NOT NULL,
    metadata        JSONB       NOT NULL DEFAULT '{}',
    embedding       VECTOR(384),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (id, tenant_id)
);

CREATE TABLE IF NOT EXISTS agents (
    actor_id        UUID        PRIMARY KEY REFERENCES actors(id) ON DELETE CASCADE,
    status          TEXT        NOT NULL, -- online, offline
    description     TEXT        NOT NULL,
    secret          TEXT        NOT NULL,
    instances       INT         NOT NULL DEFAULT 0,
    skills          JSONB       NOT NULL DEFAULT '[]'
);

CREATE UNIQUE INDEX idx_actors_tenant_external_id
ON actors (tenant_id, external_id)
WHERE external_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_actors_tenant_id
ON actors (tenant_id, id DESC);

CREATE INDEX IF NOT EXISTS idx_actors_tenant_role_id
ON actors (tenant_id, role, id DESC);

CREATE INDEX IF NOT EXISTS idx_actors_embedding
ON actors USING hnsw (embedding vector_cosine_ops);
