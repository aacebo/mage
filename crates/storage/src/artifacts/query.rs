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
    pub message_id: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by_id: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

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

    pub fn message(mut self, value: uuid::Uuid) -> Self {
        self.message_id = Some(value);
        self
    }

    pub fn task(mut self, value: uuid::Uuid) -> Self {
        self.task_id = Some(value);
        self
    }

    pub fn created_by(mut self, value: uuid::Uuid) -> Self {
        self.created_by_id = Some(value);
        self
    }

    pub fn name(mut self, value: impl std::fmt::Display) -> Self {
        self.name = Some(value.to_string());
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

    pub async fn exec(&self, pool: &sqlx::PgPool) -> error::Result<QueryResult<types::resources::Artifact>> {
        self.validate()?;
        let json = super::project::jsonb_build_object("artifacts");
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(format!("SELECT {json} FROM artifacts WHERE TRUE"));

        if let Some(chat_id) = self.chat_id {
            qb.push(" AND artifacts.chat_id = ").push_bind(chat_id);
        }

        if let Some(message_id) = self.message_id {
            qb.push(" AND artifacts.message_id = ").push_bind(message_id);
        }

        if let Some(task_id) = self.task_id {
            qb.push(" AND artifacts.task_id = ").push_bind(task_id);
        }

        if let Some(created_by_id) = self.created_by_id {
            qb.push(" AND artifacts.created_by_id = ").push_bind(created_by_id);
        }

        if let Some(name) = &self.name {
            qb.push(" AND artifacts.name = ").push_bind(name);
        }

        if let Some(before) = self.before {
            qb.push(" AND artifacts.created_at < ").push_bind(before);
        }

        if let Some(after) = self.after {
            qb.push(" AND artifacts.created_at > ").push_bind(after);
        }

        if let Some(cursor) = self.cursor {
            qb.push(" AND artifacts.id < ").push_bind(cursor);
        }

        qb.push(" ORDER BY artifacts.id DESC");
        qb.push(" LIMIT ").push_bind((self.limit + 1) as i64);

        let rows = qb
            .build_query_scalar::<sqlx::types::Json<types::resources::Artifact>>()
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|sqlx::types::Json(artifact)| artifact)
            .collect();

        Ok(crate::result(rows, self.limit, |artifact| artifact.id))
    }
}

impl Default for Query {
    fn default() -> Self {
        Self {
            limit: 10,
            cursor: None,
            chat_id: None,
            message_id: None,
            task_id: None,
            created_by_id: None,
            name: None,
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
