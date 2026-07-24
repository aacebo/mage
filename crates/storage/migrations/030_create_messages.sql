CREATE TABLE IF NOT EXISTS messages (
    id              UUID        PRIMARY KEY,
    chat_id         UUID        NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    content         JSONB       NOT NULL DEFAULT '[]',
    embedding       VECTOR(384),
    metadata        JSONB       NOT NULL DEFAULT '{}',
    created_by_id   UUID        NOT NULL REFERENCES actors(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_messages_created_at
ON messages(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_messages_chat_id
ON messages(chat_id, id DESC);

CREATE INDEX IF NOT EXISTS idx_messages_created_by_id
ON messages(created_by_id, id DESC);

CREATE INDEX IF NOT EXISTS idx_messages_embedding
ON messages USING hnsw (embedding vector_cosine_ops);
