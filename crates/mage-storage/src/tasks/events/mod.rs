use mage_error::Result;
use sqlx::PgPool;
use sqlx::types::Json;

use crate::QueryResult;

pub mod project;
pub mod query;
pub use query::Query;

pub struct TaskEventStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> TaskEventStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_id(&self, id: uuid::Uuid) -> Result<Option<mage_types::tasks::TaskEvent>> {
        let query = format!(
            "SELECT {} FROM task_events WHERE task_events.id = $1",
            project::jsonb_build_object("task_events")
        );
        let event = sqlx::query_scalar::<_, Json<mage_types::tasks::TaskEvent>>(sqlx::AssertSqlSafe(query))
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        Ok(event.map(|Json(event)| event))
    }

    pub async fn get(&self, query: Query) -> Result<QueryResult<mage_types::tasks::TaskEvent, u64>> {
        query.exec(self.pool).await
    }

    pub async fn create(&self, event: mage_types::tasks::TaskEvent) -> Result<mage_types::tasks::TaskEvent> {
        if event.sequence == 0 {
            return Err(mage_error::bad_request("task event sequence must be greater than zero"));
        }

        let sequence = i64::try_from(event.sequence)
            .map_err(|_| mage_error::bad_request("task event sequence exceeds PostgreSQL BIGINT"))?;
        let data = match &event.data {
            mage_types::tasks::TaskEventData::Custom(value) => serde_json::Value::String(value.clone()),
            _ => serde_json::json!({}),
        };

        sqlx::query(
            r#"
            INSERT INTO task_events (
                id, tenant_id, task_id, sequence, type, data,
                created_by_id, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            "#,
        )
        .bind(event.id)
        .bind(event.tenant_id)
        .bind(event.task_id)
        .bind(sequence)
        .bind(event.data.as_str())
        .bind(Json(data))
        .bind(event.created_by.id)
        .execute(self.pool)
        .await?;

        self.get_by_id(event.id)
            .await?
            .ok_or_else(|| mage_error::Error::from(sqlx::Error::RowNotFound))
    }
}
