CREATE TABLE IF NOT EXISTS artifacts (
    id              UUID                PRIMARY KEY,
    chat_id         UUID                NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    message_id      UUID                REFERENCES messages(id) ON DELETE CASCADE,
    task_id         UUID                REFERENCES tasks(id) ON DELETE CASCADE,
    name            TEXT                NOT NULL,
    content         JSONB               NOT NULL DEFAULT '[]',
    embedding       VECTOR(384),
    metadata        JSONB               NOT NULL DEFAULT '{}',
    created_by_id   UUID                REFERENCES actors(id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ         NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ         NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_artifacts_name_id
ON artifacts(name, id DESC);

CREATE INDEX IF NOT EXISTS idx_artifacts_chat_id
ON artifacts(chat_id, id DESC);

CREATE INDEX IF NOT EXISTS idx_artifacts_message_id
ON artifacts(message_id, id DESC)
WHERE message_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_artifacts_task_id
ON artifacts(task_id, id DESC)
WHERE task_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_artifacts_created_by_id
ON artifacts(created_by_id, id DESC)
WHERE created_by_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_artifacts_embedding
ON artifacts USING hnsw(embedding vector_cosine_ops);
