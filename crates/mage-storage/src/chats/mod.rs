use mage_error::Result;
use sqlx::PgPool;
use sqlx::types::Json;

use crate::QueryResult;

pub mod project;
pub mod query;
pub use query::Query;

pub struct ChatStorage<'a> {
    pool: &'a PgPool,
}

impl<'a> ChatStorage<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_by_id(&self, id: uuid::Uuid) -> Result<Option<mage_types::chats::Chat>> {
        let query = format!(
            "SELECT {} FROM chats WHERE chats.id = $1",
            project::jsonb_build_object("chats")
        );
        let chat = sqlx::query_scalar::<_, Json<mage_types::chats::Chat>>(sqlx::AssertSqlSafe(query))
            .bind(id)
            .fetch_optional(self.pool)
            .await?;

        Ok(chat.map(|Json(chat)| chat))
    }

    pub async fn get(&self, query: Query) -> Result<QueryResult<mage_types::chats::Chat>> {
        query.exec(self.pool).await
    }

    pub async fn get_open_for_actor(
        &self,
        id: uuid::Uuid,
        tenant_id: uuid::Uuid,
        actor_id: uuid::Uuid,
    ) -> Result<Option<mage_types::chats::Chat>> {
        let query = format!(
            r#"
            SELECT {}
            FROM chats
            WHERE chats.id = $1
              AND chats.tenant_id = $2
              AND chats.closed_at IS NULL
              AND EXISTS (
                  SELECT 1
                  FROM chat_actors
                  WHERE chat_actors.chat_id = chats.id
                    AND chat_actors.actor_id = $3
              )
            "#,
            project::jsonb_build_object("chats")
        );

        let chat = sqlx::query_scalar::<_, Json<mage_types::chats::Chat>>(sqlx::AssertSqlSafe(query))
            .bind(id)
            .bind(tenant_id)
            .bind(actor_id)
            .fetch_optional(self.pool)
            .await?;

        Ok(chat.map(|Json(chat)| chat))
    }

    pub async fn create(&self, chat: mage_types::chats::Chat) -> Result<mage_types::chats::Chat> {
        sqlx::query(
            r#"
            INSERT INTO chats (id, tenant_id, name, created_by_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            "#,
        )
        .bind(chat.id)
        .bind(chat.tenant_id)
        .bind(&chat.name)
        .bind(chat.created_by.id)
        .execute(self.pool)
        .await?;

        self.get_by_id(chat.id)
            .await?
            .ok_or_else(|| mage_error::Error::from(sqlx::Error::RowNotFound))
    }

    pub async fn update(&self, chat: mage_types::chats::Chat) -> Result<mage_types::chats::Chat> {
        let result = sqlx::query(
            r#"
            UPDATE chats
            SET name = $2,
                closed_at = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(chat.id)
        .bind(&chat.name)
        .bind(chat.closed_at)
        .execute(self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound.into());
        }

        self.get_by_id(chat.id)
            .await?
            .ok_or_else(|| mage_error::Error::from(sqlx::Error::RowNotFound))
    }

    pub async fn set_actors(
        &self,
        chat_id: uuid::Uuid,
        actor_ids: impl IntoIterator<Item = uuid::Uuid>,
    ) -> Result<mage_types::chats::Chat> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM chat_actors WHERE chat_id = $1")
            .bind(chat_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            r#"
            INSERT INTO chat_actors (chat_id, actor_id, created_at)
            SELECT $1, actor_id, NOW()
            FROM (
                SELECT DISTINCT actor_id
                FROM UNNEST($2::UUID[]) AS actor(actor_id)
            ) actors
            "#,
        )
        .bind(chat_id)
        .bind(actor_ids.into_iter().collect::<Vec<_>>())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        self.get_by_id(chat_id)
            .await?
            .ok_or_else(|| mage_error::Error::from(sqlx::Error::RowNotFound))
    }

    pub async fn delete(&self, id: uuid::Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM chats WHERE id = $1")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
