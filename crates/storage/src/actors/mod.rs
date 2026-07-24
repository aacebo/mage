use error::Result;
use pgvector::Vector;
use sqlx::PgPool;
use sqlx::types::Json;

use crate::{QueryResult, SearchOptions, SearchResult, search};

pub mod project;
pub mod query;

pub struct ActorStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> ActorStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_id(&self, id: uuid::Uuid) -> Result<Option<types::actors::Actor>> {
        let query = format!(
            "SELECT {} FROM actors WHERE actors.id = $1",
            project::jsonb_build_object("actors")
        );
        let actor = sqlx::query_scalar::<_, Json<types::actors::Actor>>(&query)
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        Ok(actor.map(|Json(actor)| actor))
    }

    pub async fn get(&self, query: query::Query) -> Result<QueryResult<types::actors::Actor>> {
        query.exec(self.pool).await
    }

    pub async fn get_by_external_id(&self, tenant_id: uuid::Uuid, external_id: String) -> Result<Option<types::actors::Actor>> {
        let query = format!(
            r#"SELECT {}
            FROM actors
            WHERE actors.tenant_id = $1
            AND actors.external_id = $2"#,
            project::jsonb_build_object("actors"),
        );

        let actor = sqlx::query_scalar::<_, Json<types::actors::Actor>>(&query)
            .bind(tenant_id)
            .bind(external_id)
            .fetch_optional(self.pool)
            .await?;

        Ok(actor.map(|Json(actor)| actor))
    }

    pub async fn get_secret(&self, id: uuid::Uuid) -> Result<Option<String>> {
        let secret = sqlx::query_scalar("SELECT secret FROM agents WHERE actor_id = $1")
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        Ok(secret)
    }

    pub async fn search(
        &self,
        tenant_id: uuid::Uuid,
        embedding: Vec<f32>,
        options: SearchOptions,
    ) -> Result<Vec<SearchResult<types::actors::Actor>>> {
        let role = options.role.map(types::actors::Role::as_str);
        let (embedding, limit, min_similarity) = search::prepare(embedding, options)?;
        let projection = project::jsonb_build_object("actors");
        let query = format!(
            r#"
            WITH nearest AS MATERIALIZED (
                SELECT {projection} AS entity,
                       actors.embedding <=> $2 AS distance
                FROM actors
                WHERE actors.tenant_id = $1
                  AND actors.embedding IS NOT NULL
                  AND ($5::TEXT IS NULL OR actors.role = $5)
                ORDER BY actors.embedding <=> $2
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
        let rows = sqlx::query_as::<_, (Json<types::actors::Actor>, f64)>(&query)
            .bind(tenant_id)
            .bind(embedding)
            .bind(limit)
            .bind(min_similarity)
            .bind(role)
            .fetch_all(&mut *tx)
            .await?;
        tx.commit().await?;

        Ok(rows
            .into_iter()
            .map(|(Json(entity), similarity)| SearchResult { entity, similarity })
            .collect())
    }

    pub async fn create(&self, actor: types::actors::Actor) -> Result<types::actors::Actor> {
        let mut tx = self.pool.begin().await?;
        let embedding = actor.embedding.clone().map(Vector::from);

        sqlx::query(
            r#"
            INSERT INTO actors (
                id, tenant_id, external_id, role, name, metadata,
                embedding, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
            "#,
        )
        .bind(actor.id)
        .bind(actor.tenant_id)
        .bind(&actor.external_id)
        .bind(actor.role.as_str())
        .bind(&actor.name)
        .bind(Json(&actor.metadata))
        .bind(embedding)
        .execute(&mut *tx)
        .await?;

        if let Some(agent) = &actor.agent {
            sqlx::query(
                r#"
                INSERT INTO agents (actor_id, status, description, secret, instances, skills)
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
            )
            .bind(actor.id)
            .bind(agent.status.as_str())
            .bind(&agent.description)
            .bind(&agent.secret)
            .bind(agent.instances as i32)
            .bind(Json(&agent.skills))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(self.get_by_id(actor.id).await?.ok_or(sqlx::Error::RowNotFound)?)
    }

    pub async fn update(&self, actor: types::actors::Actor) -> Result<types::actors::Actor> {
        let mut tx = self.pool.begin().await?;
        let embedding = actor.embedding.clone().map(Vector::from);
        let result = sqlx::query(
            r#"
            UPDATE actors
            SET tenant_id = $2,
                external_id = $3,
                role = $4,
                name = $5,
                metadata = $6,
                embedding = $7,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(actor.id)
        .bind(actor.tenant_id)
        .bind(&actor.external_id)
        .bind(actor.role.as_str())
        .bind(&actor.name)
        .bind(Json(&actor.metadata))
        .bind(embedding)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound.into());
        }

        if let Some(agent) = &actor.agent {
            sqlx::query(
                r#"
                INSERT INTO agents (actor_id, status, description, instances, skills)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (actor_id) DO UPDATE
                SET status = EXCLUDED.status,
                    description = EXCLUDED.description,
                    instances = EXCLUDED.instances,
                    skills = EXCLUDED.skills
                "#,
            )
            .bind(actor.id)
            .bind(agent.status.as_str())
            .bind(&agent.description)
            .bind(agent.instances as i32)
            .bind(Json(&agent.skills))
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query("DELETE FROM agents WHERE actor_id = $1")
                .bind(actor.id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(self.get_by_id(actor.id).await?.ok_or(sqlx::Error::RowNotFound)?)
    }

    pub async fn update_embedding(&self, id: uuid::Uuid, embedding: Vec<f32>) -> Result<types::actors::Actor> {
        let result = sqlx::query(
            r#"
            UPDATE actors
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

        Ok(self.get_by_id(id).await?.ok_or(sqlx::Error::RowNotFound)?)
    }

    pub async fn update_secret(&self, id: uuid::Uuid, value: impl std::fmt::Display) -> Result<types::actors::Actor> {
        let result = sqlx::query(
            r#"
            UPDATE agents
            SET secret = $2,
                updated_at = NOW()
            WHERE actor_id = $1
            "#,
        )
        .bind(id)
        .bind(value.to_string())
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound.into());
        }

        Ok(self.get_by_id(id).await?.ok_or(sqlx::Error::RowNotFound)?)
    }

    pub async fn connect(&self, id: uuid::Uuid) -> Result<Option<types::actors::Actor>> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query(
            r#"
            UPDATE agents
            SET instances = instances + 1,
                status = 'online'
            WHERE actor_id = $1
            "#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(None);
        }

        sqlx::query("UPDATE actors SET updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        self.get_by_id(id).await
    }

    pub async fn disconnect(&self, id: uuid::Uuid) -> Result<Option<types::actors::Actor>> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query(
            r#"
            UPDATE agents
            SET instances = GREATEST(instances - 1, 0),
                status = CASE
                    WHEN instances <= 1 THEN 'offline'
                    ELSE 'online'
                END
            WHERE actor_id = $1
            "#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Ok(None);
        }

        sqlx::query("UPDATE actors SET updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        self.get_by_id(id).await
    }

    pub async fn delete(&self, id: uuid::Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM actors WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
