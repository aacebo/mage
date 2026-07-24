pub fn jsonb_build_object(alias: &str) -> String {
    format!(
        r#"
        jsonb_build_object(
            'id', {alias}.id,
            'type', {alias}.type,
            'label', {alias}.label,
            'text', {alias}.text,
            'score', {alias}.score,
            'spans', {alias}.spans,
            'created_at', {alias}.created_at
        )
        "#
    )
}
