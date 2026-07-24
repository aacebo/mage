pub fn jsonb_build_object(alias: &str) -> String {
    format!(
        r#"
        jsonb_build_object(
            'id', {alias}.id,
            'tenant_id', {alias}.tenant_id,
            'trace_id', {alias}.trace_id,
            'key', {alias}.key,
            'data', {alias}.data,
            'created_at', {alias}.created_at
        )
        "#
    )
}
