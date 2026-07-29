use mage_error::Result;
use pgvector::Vector;
use sqlx::PgPool;
use sqlx::types::Json;

use crate::{QueryResult, SearchOptions, SearchResult, search};

pub mod project;
pub mod query;
pub use query::Query;

pub struct ArtifactStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> ArtifactStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_id(&self, id: uuid::Uuid) -> Result<Option<mage_types::resources::Artifact>> {
        let query = format!(
            "SELECT {} FROM artifacts WHERE artifacts.id = $1",
            project::jsonb_build_object("artifacts")
        );
        let artifact = sqlx::query_scalar::<_, Json<mage_types::resources::Artifact>>(sqlx::AssertSqlSafe(query))
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        Ok(artifact.map(|Json(artifact)| artifact))
    }

    pub async fn get(&self, query: Query) -> Result<QueryResult<mage_types::resources::Artifact>> {
        query.exec(self.pool).await
    }

    pub async fn search(
        &self,
        tenant_id: uuid::Uuid,
        embedding: Vec<f32>,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult<mage_types::resources::Artifact>>> {
        let (embedding, limit, min_similarity) = search::prepare(embedding, options)?;
        let projection = project::jsonb_build_object("artifacts");
        let query = format!(
            r#"
            WITH nearest AS MATERIALIZED (
                SELECT {projection} AS entity,
                       artifacts.embedding <=> $2 AS distance
                FROM artifacts
                WHERE artifacts.embedding IS NOT NULL
                  AND EXISTS (
                      SELECT 1
                      FROM chats
                      WHERE chats.id = artifacts.chat_id
                        AND chats.tenant_id = $1
                  )
                ORDER BY artifacts.embedding <=> $2
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
        let rows = sqlx::query_as::<_, (Json<mage_types::resources::Artifact>, f64)>(sqlx::AssertSqlSafe(query))
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

    pub async fn create(
        &self,
        chat_id: uuid::Uuid,
        message_id: Option<uuid::Uuid>,
        task_id: Option<uuid::Uuid>,
        artifact: mage_types::resources::Artifact,
    ) -> Result<mage_types::resources::Artifact> {
        let embedding = artifact.embedding.clone().map(Vector::from);
        sqlx::query(
            r#"
            INSERT INTO artifacts (
                id, chat_id, message_id, task_id, name, content, embedding,
                metadata, created_by_id, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())
            "#,
        )
        .bind(artifact.id)
        .bind(chat_id)
        .bind(message_id)
        .bind(task_id)
        .bind(&artifact.name)
        .bind(Json(&artifact.content))
        .bind(embedding)
        .bind(Json(&artifact.metadata))
        .bind(artifact.created_by.id)
        .execute(self.pool)
        .await?;

        self.get_by_id(artifact.id)
            .await?
            .ok_or_else(|| mage_error::Error::from(sqlx::Error::RowNotFound))
    }

    pub async fn update(&self, artifact: mage_types::resources::Artifact) -> Result<mage_types::resources::Artifact> {
        let embedding = artifact.embedding.clone().map(Vector::from);
        let result = sqlx::query(
            r#"
            UPDATE artifacts
            SET name = $2,
                content = $3,
                embedding = $4,
                metadata = $5,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(artifact.id)
        .bind(&artifact.name)
        .bind(Json(&artifact.content))
        .bind(embedding)
        .bind(Json(&artifact.metadata))
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound.into());
        }

        self.get_by_id(artifact.id)
            .await?
            .ok_or_else(|| mage_error::Error::from(sqlx::Error::RowNotFound))
    }

    pub async fn update_embedding(&self, id: uuid::Uuid, embedding: Vec<f32>) -> Result<mage_types::resources::Artifact> {
        let result = sqlx::query(
            r#"
            UPDATE artifacts
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
        let result = sqlx::query("DELETE FROM artifacts WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
