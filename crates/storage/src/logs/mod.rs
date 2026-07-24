pub mod project;
pub mod query;

use error::Result;
pub use query::Query;
use sqlx::PgPool;
use sqlx::types::Json;

use crate::QueryResult;

pub struct LogStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> LogStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_id(&self, id: uuid::Uuid) -> Result<Option<types::logs::Log>> {
        let query = format!("SELECT {} FROM logs WHERE id = $1", project::jsonb_build_object("logs"));
        let log = sqlx::query_scalar::<_, Json<types::logs::Log>>(sqlx::AssertSqlSafe(query))
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        Ok(log.map(|Json(log)| log))
    }

    pub async fn get(&self, query: Query) -> Result<QueryResult<types::logs::Log>> {
        query.exec(self.pool).await
    }

    pub async fn create(&self, log: types::logs::Log) -> Result<types::logs::Log> {
        sqlx::query(
            r#"
            INSERT INTO logs (
                id, trace_id, tenant_id, task_id, level,
                source, message, fields, created_by_id, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            "#,
        )
        .bind(log.id)
        .bind(log.trace_id)
        .bind(log.tenant_id)
        .bind(log.task_id)
        .bind(log.level.as_str())
        .bind(log.source)
        .bind(log.message)
        .bind(Json(log.fields))
        .bind(log.created_by.id)
        .execute(self.pool)
        .await?;

        self.get_by_id(log.id)
            .await?
            .ok_or_else(|| error::Error::from(sqlx::Error::RowNotFound))
    }
}
