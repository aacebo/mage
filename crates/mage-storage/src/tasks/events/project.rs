pub fn jsonb_build_object(alias: &str) -> String {
    let created_by = crate::actors::project::partial("created_by");

    format!(
        r#"
        jsonb_build_object(
            'id', {alias}.id,
            'tenant_id', {alias}.tenant_id,
            'task_id', {alias}.task_id,
            'sequence', {alias}.sequence,
            'type', {alias}.type,
            'created_by', (
                SELECT {created_by}
                FROM actors created_by
                WHERE created_by.id = {alias}.created_by_id
            ),
            'created_at', {alias}.created_at
        ) || CASE
            WHEN {alias}.type = 'custom' THEN jsonb_build_object('data', {alias}.data)
            ELSE '{{}}'::jsonb
        END
        "#
    )
}
