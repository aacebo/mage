pub fn jsonb_build_object(alias: &str) -> String {
    let created_by = crate::actors::project::partial("created_by");

    format!(
        r#"
        jsonb_build_object(
            'id', {alias}.id,
            'trace_id', {alias}.trace_id,
            'tenant_id', {alias}.tenant_id,
            'task_id', {alias}.task_id,
            'level', {alias}.level,
            'source', {alias}.source,
            'message', {alias}.message,
            'fields', {alias}.fields,
            'created_by', (
                SELECT {created_by}
                FROM actors created_by
                WHERE created_by.id = {alias}.created_by_id
            ),
            'created_at', {alias}.created_at
        )
        "#
    )
}
