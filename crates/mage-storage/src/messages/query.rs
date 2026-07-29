use serde_valid::Validate;

use crate::QueryResult;

pub fn new() -> Query {
    Query::default()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct Query {
    #[validate(minimum = 1)]
    #[validate(maximum = 100)]
    #[serde(default = "default::limit")]
    pub limit: usize,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_id: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by_id: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<chrono::DateTime<chrono::Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<chrono::DateTime<chrono::Utc>>,
}

impl Query {
    pub fn limit(mut self, value: usize) -> Self {
        self.limit = value;
        self
    }

    pub fn cursor(mut self, value: uuid::Uuid) -> Self {
        self.cursor = Some(value);
        self
    }

    pub fn chat(mut self, value: uuid::Uuid) -> Self {
        self.chat_id = Some(value);
        self
    }

    pub fn created_by(mut self, value: uuid::Uuid) -> Self {
        self.created_by_id = Some(value);
        self
    }

    pub fn before(mut self, value: chrono::DateTime<chrono::Utc>) -> Self {
        self.before = Some(value);
        self
    }

    pub fn after(mut self, value: chrono::DateTime<chrono::Utc>) -> Self {
        self.after = Some(value);
        self
    }

    pub fn between(self, after: chrono::DateTime<chrono::Utc>, before: chrono::DateTime<chrono::Utc>) -> Self {
        self.after(after).before(before)
    }

    pub async fn exec(&self, pool: &sqlx::PgPool) -> mage_error::Result<QueryResult<mage_types::chats::Message>> {
        self.validate()?;
        let json = super::project::jsonb_build_object("messages");
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(format!("SELECT {json} FROM messages WHERE TRUE"));

        if let Some(chat_id) = self.chat_id {
            qb.push(" AND messages.chat_id = ").push_bind(chat_id);
        }

        if let Some(created_by_id) = self.created_by_id {
            qb.push(" AND messages.created_by_id = ").push_bind(created_by_id);
        }

        if let Some(before) = self.before {
            qb.push(" AND messages.created_at < ").push_bind(before);
        }

        if let Some(after) = self.after {
            qb.push(" AND messages.created_at > ").push_bind(after);
        }

        if let Some(cursor) = self.cursor {
            qb.push(" AND messages.id < ").push_bind(cursor);
        }

        qb.push(" ORDER BY messages.id DESC");
        qb.push(" LIMIT ").push_bind((self.limit + 1) as i64);

        let rows = qb
            .build_query_scalar::<sqlx::types::Json<mage_types::chats::Message>>()
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|sqlx::types::Json(message)| message)
            .collect();

        Ok(crate::result(rows, self.limit, |message| message.id))
    }
}

impl Default for Query {
    fn default() -> Self {
        Self {
            limit: 10,
            cursor: None,
            chat_id: None,
            created_by_id: None,
            before: None,
            after: None,
        }
    }
}

mod default {
    pub fn limit() -> usize {
        10
    }
}
