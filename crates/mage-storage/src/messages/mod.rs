use mage_error::Result;
use pgvector::Vector;
use sqlx::PgPool;
use sqlx::types::Json;

use crate::{QueryResult, SearchOptions, SearchResult, search};

pub mod project;
pub mod query;
pub use query::Query;

pub struct MessageStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> MessageStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_id(&self, id: uuid::Uuid) -> Result<Option<mage_types::chats::Message>> {
        let query = format!(
            "SELECT {} FROM messages WHERE messages.id = $1",
            project::jsonb_build_object("messages")
        );

        let message = sqlx::query_scalar::<_, Json<mage_types::chats::Message>>(sqlx::AssertSqlSafe(query))
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        Ok(message.map(|Json(message)| message))
    }

    pub async fn get(&self, query: Query) -> Result<QueryResult<mage_types::chats::Message>> {
        query.exec(self.pool).await
    }

    pub async fn get_by_task(&self, task_id: uuid::Uuid) -> Result<Option<mage_types::chats::Message>> {
        let query = format!(
            r#"
            SELECT {}
            FROM messages
            JOIN tasks ON tasks.message_id = messages.id
            WHERE tasks.id = $1
            "#,
            project::jsonb_build_object("messages")
        );

        let message = sqlx::query_scalar::<_, Json<mage_types::chats::Message>>(sqlx::AssertSqlSafe(query))
            .bind(task_id)
            .fetch_optional(self.pool)
            .await?;

        Ok(message.map(|Json(message)| message))
    }

    pub async fn search(
        &self,
        tenant_id: uuid::Uuid,
        embedding: Vec<f32>,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult<mage_types::chats::Message>>> {
        let (embedding, limit, min_similarity) = search::prepare(embedding, options)?;
        let projection = project::jsonb_build_object("messages");
        let query = format!(
            r#"
            WITH nearest AS MATERIALIZED (
                SELECT {projection} AS entity,
                       messages.embedding <=> $2 AS distance
                FROM messages
                WHERE messages.embedding IS NOT NULL
                  AND EXISTS (
                      SELECT 1
                      FROM chats
                      WHERE chats.id = messages.chat_id
                        AND chats.tenant_id = $1
                  )
                ORDER BY messages.embedding <=> $2
                LIMIT $3
            )
            SELECT entity, 1.0 - distance AS similarity
            FROM nearest
            WHERE distance <= 1.0 - $4
            ORDER BY distance
            "#,
        );
        let mut tx = self.pool.begin().await?;
        sqlx::query("SET LOCAL hnsw.iterative_scan = strict_order")
            .execute(&mut *tx)
            .await?;
        let rows = sqlx::query_as::<_, (Json<mage_types::chats::Message>, f64)>(sqlx::AssertSqlSafe(query))
            .bind(tenant_id)
            .bind(embedding)
            .bind(limit)
            .bind(min_similarity)
            .fetch_all(&mut *tx)
            .await?;
        tx.commit().await?;

        Ok(rows
            .into_iter()
            .map(|(Json(entity), similarity)| SearchResult { entity, similarity })
            .collect())
    }

    pub async fn create(&self, message: mage_types::chats::Message) -> Result<mage_types::chats::Message> {
        let embedding = message.embedding.clone().map(Vector::from);
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO messages (
                id, chat_id, content, metadata, embedding, created_by_id, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
            "#,
        )
        .bind(message.id)
        .bind(message.chat.id)
        .bind(Json(&message.content))
        .bind(Json(&message.metadata))
        .bind(embedding)
        .bind(message.created_by.id)
        .execute(&mut *tx)
        .await?;

        sqlx::query("UPDATE chats SET updated_at = NOW() WHERE id = $1")
            .bind(message.chat.id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        self.get_by_id(message.id)
            .await?
            .ok_or_else(|| mage_error::Error::from(sqlx::Error::RowNotFound))
    }

    pub async fn update(&self, message: mage_types::chats::Message) -> Result<mage_types::chats::Message> {
        let embedding = message.embedding.clone().map(Vector::from);
        let result = sqlx::query(
            r#"
            UPDATE messages
            SET content = $2,
                metadata = $3,
                embedding = $4,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(message.id)
        .bind(Json(&message.content))
        .bind(Json(&message.metadata))
        .bind(embedding)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound.into());
        }

        self.get_by_id(message.id)
            .await?
            .ok_or_else(|| mage_error::Error::from(sqlx::Error::RowNotFound))
    }

    pub async fn update_embedding(&self, id: uuid::Uuid, embedding: Vec<f32>) -> Result<mage_types::chats::Message> {
        let result = sqlx::query(
            r#"
            UPDATE messages
            SET embedding = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(Vector::from(embedding))
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound.into());
        }

        self.get_by_id(id)
            .await?
            .ok_or_else(|| mage_error::Error::from(sqlx::Error::RowNotFound))
    }

    pub async fn delete(&self, id: uuid::Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM messages WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
