use mage_error::Result;
use sqlx::PgPool;
use sqlx::types::Json;

use crate::QueryResult;

pub mod project;
pub mod query;
pub use query::Query;

pub struct EventStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> EventStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_id(&self, id: uuid::Uuid) -> Result<Option<mage_types::events::Event>> {
        let query = format!(
            "SELECT {} FROM events WHERE events.id = $1",
            project::jsonb_build_object("events")
        );

        let event = sqlx::query_scalar::<_, Json<mage_types::events::Event>>(sqlx::AssertSqlSafe(query))
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        Ok(event.map(|Json(event)| event))
    }

    pub async fn get(&self, query: Query) -> Result<QueryResult<mage_types::events::Event>> {
        query.exec(self.pool).await
    }

    pub async fn create(
        &self,
        actor_id: Option<uuid::Uuid>,
        chat_id: Option<uuid::Uuid>,
        message_id: Option<uuid::Uuid>,
        task_id: Option<uuid::Uuid>,
        event: mage_types::events::Event,
    ) -> Result<mage_types::events::Event> {
        sqlx::query(
            r#"
            INSERT INTO events (
                id, trace_id, tenant_id, actor_id, chat_id, message_id, task_id,
                key, data, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            "#,
        )
        .bind(event.id)
        .bind(event.trace_id)
        .bind(event.tenant_id)
        .bind(actor_id)
        .bind(chat_id)
        .bind(message_id)
        .bind(task_id)
        .bind(&event.key)
        .bind(Json(&event.data))
        .execute(self.pool)
        .await?;

        self.get_by_id(event.id)
            .await?
            .ok_or_else(|| mage_error::Error::from(sqlx::Error::RowNotFound))
    }
}
