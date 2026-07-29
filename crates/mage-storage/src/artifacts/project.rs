pub fn jsonb_build_object(alias: &str) -> String {
    let created_by = crate::actors::project::jsonb_build_object("created_by");
    format!(
        r#"
        jsonb_build_object(
            'id', {alias}.id,
            'name', {alias}.name,
            'content', {alias}.content,
            'embedding', CASE
                WHEN {alias}.embedding IS NULL THEN NULL
                ELSE ({alias}.embedding::text)::jsonb
            END,
            'metadata', {alias}.metadata,
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
