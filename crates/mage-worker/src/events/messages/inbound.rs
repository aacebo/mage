use crate::{context::EventContext, routing};

pub async fn run(ctx: &EventContext<'_>, message: &mage_types::chats::InboundMessage) -> mage_error::Result<()> {
    let result: mage_error::Result<Option<usize>> = async {
        let artifacts = tokio::task::block_in_place(move || {
            mage_ai::pipelines::embeddings(mage_ai::pipelines::TextArgs {
                text: vec![message.content.to_string()],
                ..Default::default()
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

        let dimensions = vector.len();

        if let Some(chat_id) = message.chat_id {
            let chat = ctx
                .storage()
                .chats()
                .get_open_for_actor(chat_id, message.tenant_id, message.sent_by.id)
                .await?
                .ok_or_else(|| mage_error::bad_request("chat is unavailable for this sender"))?;

            let message = ctx
                .storage()
                .messages()
                .create(mage_types::chats::Message {
                    id: uuid::Uuid::now_v7(),
                    chat: chat.into(),
                    content: message.content.clone(),
                    metadata: message.metadata.clone(),
                    embedding: Some(vector),
                    created_by: message.sent_by.clone(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                })
                .await?;

            ctx.enqueue("message.create", message).await?;
            return Ok(Some(dimensions));
        }

        let policy = ctx.routing();
        let options = mage_storage::SearchOptions::new(policy.candidate_limit, -1.0)?.with_role(mage_types::actors::Role::Agent);
        let candidates = ctx
            .storage()
            .actors()
            .search(ctx.event().tenant_id, vector.clone(), options)
            .await?;

        let selected_agents = match policy.decide(candidates) {
            routing::Decision::Selected { agents, candidates } => {
                log_routing_decision(policy, "selected", None, &candidates, &agents);
                agents
            }
            routing::Decision::NoRoute { reason, candidates } => {
                log_routing_decision(policy, "no_route", Some(reason.as_str()), &candidates, &[]);
                return Ok(None);
            }
        };

        let chat = ctx
            .storage()
            .chats()
            .create(mage_types::chats::Chat {
                id: uuid::Uuid::now_v7(),
                tenant_id: message.tenant_id,
                name: message.subject.clone(),
                created_by: message.sent_by.clone(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                closed_at: None,
            })
            .await?;

        ctx.enqueue("chat.create", chat.clone()).await?;
        let mut member_ids = vec![message.sent_by.id];
        member_ids.extend(selected_agents.iter().map(|agent| agent.entity.id));
        ctx.storage().chats().set_actors(chat.id, member_ids.clone()).await?;
        ctx.enqueue(
            "chat.members",
            mage_types::chats::ChatMembers {
                chat_id: chat.id,
                actor_ids: member_ids,
            },
        )
        .await?;

        let message = ctx
            .storage()
            .messages()
            .create(mage_types::chats::Message {
                id: uuid::Uuid::now_v7(),
                chat: chat.into(),
                content: message.content.clone(),
                metadata: message.metadata.clone(),
                embedding: Some(vector),
                created_by: message.sent_by.clone(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
            .await?;

        ctx.enqueue("message.create", message).await?;
        Ok(Some(dimensions))
    }
    .await;

    match result {
        Ok(None) => ctx.ack().await?,
        Ok(Some(_)) => ctx.ack().await?,
        Err(error) => {
            tracing::error!(%error, "failed to route inbound message; requeuing event");
            ctx.nack().await?;
        }
    }

    Ok(())
}

fn log_routing_decision(
    policy: routing::Policy,
    outcome: &'static str,
    reason: Option<&'static str>,
    candidates: &[mage_storage::SearchResult<mage_types::actors::Actor>],
    selected: &[mage_storage::SearchResult<mage_types::actors::Actor>],
) {
    let candidate_scores: Vec<_> = candidates
        .iter()
        .map(|candidate| format!("{}:{:.6}", candidate.entity.name, candidate.similarity))
        .collect();
    let selected_agent_ids: Vec<_> = selected.iter().map(|candidate| candidate.entity.id).collect();
    let top_score = candidates.first().map(|candidate| candidate.similarity);
    let runner_up_score = candidates.get(1).map(|candidate| candidate.similarity);
    let observed_margin = top_score.zip(runner_up_score).map(|(top, runner_up)| top - runner_up);

    tracing::info!(
        outcome,
        reason = reason.unwrap_or("none"),
        ?candidate_scores,
        ?selected_agent_ids,
        ?top_score,
        ?runner_up_score,
        ?observed_margin,
        candidate_limit = policy.candidate_limit,
        min_confidence = policy.min_confidence,
        ambiguity_margin = policy.ambiguity_margin,
        "agent routing decision"
    );
}
