use error::Result;
use sqlx::PgPool;
use sqlx::types::Json;

use crate::QueryResult;

pub mod project;
pub mod query;
pub use query::Query;

pub struct TaskStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> TaskStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_id(&self, id: uuid::Uuid) -> Result<Option<types::tasks::Task>> {
        let query = format!(
            "SELECT {} FROM tasks WHERE tasks.id = $1",
            project::jsonb_build_object("tasks")
        );
        let task = sqlx::query_scalar::<_, Json<types::tasks::Task>>(sqlx::AssertSqlSafe(query))
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        Ok(task.map(|Json(task)| task))
    }

    pub async fn get(&self, query: Query) -> Result<QueryResult<types::tasks::Task>> {
        query.exec(self.pool).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        chat_id: uuid::Uuid,
        message_id: Option<uuid::Uuid>,
        agent_id: Option<uuid::Uuid>,
        task: types::tasks::Task,
    ) -> Result<types::tasks::Task> {
        sqlx::query(
            r#"
            INSERT INTO tasks (
                id, trace_id, parent_id, chat_id, message_id, agent_id, name,
                status, input, output, error, attempts, max_attempts,
                started_at, ended_at, created_at, updated_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7,
                $8, $9, $10, $11, $12, $13,
                $14, $15, NOW(), NOW()
            )
            "#,
        )
        .bind(task.id)
        .bind(task.trace_id)
        .bind(task.parent_id)
        .bind(chat_id)
        .bind(message_id)
        .bind(agent_id)
        .bind(&task.name)
        .bind(task.status.as_str())
        .bind(&task.input)
        .bind(&task.output)
        .bind(&task.error)
        .bind(task.attempts)
        .bind(task.max_attempts)
        .bind(task.started_at)
        .bind(task.ended_at)
        .execute(self.pool)
        .await?;

        self.get_by_id(task.id)
            .await?
            .ok_or_else(|| error::Error::from(sqlx::Error::RowNotFound))
    }

    pub async fn update(&self, task: types::tasks::Task) -> Result<types::tasks::Task> {
        let result = sqlx::query(
            r#"
            UPDATE tasks
            SET name = $2,
                status = $3,
                input = $4,
                output = $5,
                error = $6,
                attempts = $7,
                max_attempts = $8,
                started_at = $9,
                ended_at = $10,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(task.id)
        .bind(&task.name)
        .bind(task.status.as_str())
        .bind(&task.input)
        .bind(&task.output)
        .bind(&task.error)
        .bind(task.attempts)
        .bind(task.max_attempts)
        .bind(task.started_at)
        .bind(task.ended_at)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound.into());
        }

        self.get_by_id(task.id)
            .await?
            .ok_or_else(|| error::Error::from(sqlx::Error::RowNotFound))
    }

    pub async fn delete(&self, id: uuid::Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM tasks WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
