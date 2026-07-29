CREATE TABLE IF NOT EXISTS annotations (
    id          UUID                PRIMARY KEY,
    message_id  UUID                NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    task_id     UUID                REFERENCES tasks(id) ON DELETE CASCADE,
    type        TEXT                NOT NULL,
    label       TEXT                NOT NULL,
    text        TEXT                NOT NULL,
    score       DOUBLE PRECISION    NOT NULL,
    spans       JSONB               NOT NULL DEFAULT '[]'::jsonb,
    created_at  TIMESTAMPTZ         NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_annotations_message_id
ON annotations(message_id, id DESC);

CREATE INDEX IF NOT EXISTS idx_annotations_task_id
ON annotations(task_id, id DESC)
WHERE task_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_annotations_type_id
ON annotations(type, id DESC);

CREATE INDEX IF NOT EXISTS idx_annotations_label_id
ON annotations(label, id DESC);
