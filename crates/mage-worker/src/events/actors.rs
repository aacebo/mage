use crate::context::EventContext;

pub async fn run(ctx: &EventContext<'_>, actor_id: uuid::Uuid) -> mage_error::Result<()> {
    let result: mage_error::Result<Option<usize>> = async {
        let actor = match ctx.storage().actors().get_by_id(actor_id).await? {
            Some(actor) => actor,
            None => {
                tracing::debug!(reason = "actor_missing", "skipping actor embedding");
                return Ok(None);
            }
        };

        let mut lines = vec![format!("Name: {}", actor.name)];

        if let Some(agent) = &actor.agent
            && !agent.skills.is_empty()
        {
            lines.push(format!("Description: {}", agent.description));
            lines.push("Skills:".to_string());

            for skill in &agent.skills {
                lines.push(format!("- Name: {}", skill.name));
                lines.push(format!("  Display name: {}", skill.display_name));

                if let Some(description) = &skill.description {
                    lines.push(format!("  Description: {description}"));
                }
            }
        }

        let input = lines.join("\n");
        let artifacts = tokio::task::block_in_place(move || {
            mage_ai::pipelines::embeddings(mage_ai::pipelines::TextArgs {
                text: vec![input],
                model: mage_ai::pipelines::ModelArgs {
                    provider: None,
                    model: None,
                    base_url: None,
                    api_key: None,
                },
            })
        })?;

        if artifacts.len() != 1 {
            return Err(mage_error::ai(format!(
                "embedding pipeline returned {} artifacts; expected 1",
                artifacts.len()
            )));
        }

        let vector = artifacts
            .into_iter()
            .next()
            .and_then(|artifact| artifact.vector)
            .ok_or_else(|| mage_error::ai("embedding pipeline returned no vector"))?;

        if vector.len() != 384 {
            return Err(mage_error::ai(format!(
                "embedding pipeline returned {} dimensions; expected 384",
                vector.len()
            )));
        }

        let dimensions = vector.len();
        ctx.storage().actors().update_embedding(actor.id, vector).await?;
        Ok(Some(dimensions))
    }
    .await;

    match result {
        Ok(None) => ctx.ack().await?,
        Ok(Some(dimensions)) => {
            tracing::info!(dimensions, "stored agent embedding");
            ctx.ack().await?;
        }
        Err(error) => {
            tracing::error!(%error, "failed to create agent embedding; requeuing event");
            ctx.nack().await?;
        }
    }

    Ok(())
}
