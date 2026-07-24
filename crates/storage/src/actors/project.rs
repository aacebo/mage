pub fn agent(alias: &str) -> String {
    format!(
        r#"
        jsonb_build_object(
            'status', {alias}.status,
            'description', {alias}.description,
            'secret', {alias}.secret,
            'instances', {alias}.instances,
            'skills', {alias}.skills
        )
        "#
    )
}

pub fn partial(alias: &str) -> String {
    let agent = agent("agent");
    format!(
        r#"
        jsonb_build_object(
            'id', {alias}.id,
            'role', {alias}.role,
            'name', {alias}.name
        ) || COALESCE((
            SELECT {agent}
            FROM agents agent
            WHERE agent.actor_id = {alias}.id
        ), '{{}}'::jsonb)
        "#
    )
}

pub fn jsonb_build_object(alias: &str) -> String {
    let agent = agent("agent");
    format!(
        r#"
        jsonb_build_object(
            'id', {alias}.id,
            'tenant_id', {alias}.tenant_id,
            'external_id', {alias}.external_id,
            'role', {alias}.role,
            'name', {alias}.name,
            'metadata', {alias}.metadata,
            'embedding', CASE
                WHEN {alias}.embedding IS NULL THEN NULL
                ELSE ({alias}.embedding::text)::jsonb
            END,
            'created_at', {alias}.created_at,
            'updated_at', {alias}.updated_at
        ) || COALESCE((
            SELECT {agent}
            FROM agents agent
            WHERE agent.actor_id = {alias}.id
        ), '{{}}'::jsonb)
        "#
    )
}
