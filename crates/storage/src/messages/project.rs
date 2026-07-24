pub fn jsonb_build_object(alias: &str) -> String {
    let chat = crate::chats::project::partial("chat");
    let created_by = crate::actors::project::partial("created_by");
    format!(
        r#"
        jsonb_build_object(
            'id', {alias}.id,
            'chat', (
                SELECT {chat}
                FROM chats chat
                WHERE chat.id = {alias}.chat_id
            ),
            'content', {alias}.content,
            'metadata', {alias}.metadata,
            'embedding', CASE
                WHEN {alias}.embedding IS NULL THEN NULL
                ELSE ({alias}.embedding::text)::jsonb
            END,
            'created_by', (
                SELECT {created_by}
                FROM actors created_by
                WHERE created_by.id = {alias}.created_by_id
            ),
            'created_at', {alias}.created_at,
            'updated_at', {alias}.updated_at
        )
        "#
    )
}
