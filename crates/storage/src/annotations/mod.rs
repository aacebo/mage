use error::Result;
use sqlx::PgPool;
use sqlx::types::Json;

use crate::QueryResult;

pub mod project;
pub mod query;

pub struct AnnotationStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> AnnotationStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_id(&self, id: uuid::Uuid) -> Result<Option<types::resources::Annotation>> {
        let query = format!(
            "SELECT {} FROM annotations WHERE annotations.id = $1",
            project::jsonb_build_object("annotations")
        );
        let annotation = sqlx::query_scalar::<_, Json<types::resources::Annotation>>(&query)
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        Ok(annotation.map(|Json(annotation)| annotation))
    }

    pub async fn get(&self, query: query::Query) -> Result<QueryResult<types::resources::Annotation>> {
        query.exec(self.pool).await
    }

    pub async fn create(
        &self,
        message_id: uuid::Uuid,
        task_id: Option<uuid::Uuid>,
        annotation: types::resources::Annotation,
    ) -> Result<types::resources::Annotation> {
        sqlx::query(
            r#"
            INSERT INTO annotations (
                id, message_id, task_id, type, label, text, score, spans, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
            "#,
        )
        .bind(annotation.id)
        .bind(message_id)
        .bind(task_id)
        .bind(&annotation.r#type)
        .bind(&annotation.label)
        .bind(&annotation.text)
        .bind(annotation.score)
        .bind(Json(&annotation.spans))
        .execute(self.pool)
        .await?;

        self.get_by_id(annotation.id)
            .await?
            .ok_or_else(|| error::Error::from(sqlx::Error::RowNotFound))
    }

    pub async fn update(&self, annotation: types::resources::Annotation) -> Result<types::resources::Annotation> {
        let result = sqlx::query(
            r#"
            UPDATE annotations
            SET type = $2,
                label = $3,
                text = $4,
                score = $5,
                spans = $6
            WHERE id = $1
            "#,
        )
        .bind(annotation.id)
        .bind(&annotation.r#type)
        .bind(&annotation.label)
        .bind(&annotation.text)
        .bind(annotation.score)
        .bind(Json(&annotation.spans))
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound.into());
        }

        self.get_by_id(annotation.id)
            .await?
            .ok_or_else(|| error::Error::from(sqlx::Error::RowNotFound))
    }

    pub async fn delete(&self, id: uuid::Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM annotations WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
