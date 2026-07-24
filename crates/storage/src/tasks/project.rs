pub fn jsonb_build_object(alias: &str) -> String {
    format!(
        r#"
        jsonb_build_object(
            'id', {alias}.id,
            'trace_id', {alias}.trace_id,
            'parent_id', {alias}.parent_id,
            'name', {alias}.name,
            'status', {alias}.status,
            'input', {alias}.input,
            'output', {alias}.output,
            'error', {alias}.error,
            'attempts', {alias}.attempts,
            'max_attempts', {alias}.max_attempts,
            'started_at', {alias}.started_at,
            'ended_at', {alias}.ended_at,
            'created_at', {alias}.created_at,
            'updated_at', {alias}.updated_at
        )
        "#
    )
}
