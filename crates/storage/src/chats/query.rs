use serde_valid::Validate;

use crate::QueryResult;

pub fn new() -> Query {
    Query::default()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct Query {
    #[validate(minimum = 1)]
    #[validate(maximum = 100)]
    pub limit: usize,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by_id: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub open: Option<bool>,

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

    pub fn tenant(mut self, value: uuid::Uuid) -> Self {
        self.tenant_id = Some(value);
        self
    }

    pub fn created_by(mut self, value: uuid::Uuid) -> Self {
        self.created_by_id = Some(value);
        self
    }

    pub fn open(mut self, value: bool) -> Self {
        self.open = Some(value);
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

    pub async fn exec(&self, pool: &sqlx::PgPool) -> error::Result<QueryResult<types::chats::Chat>> {
        self.validate()?;
        let json = super::project::jsonb_build_object("chats");
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(format!("SELECT {json} FROM chats WHERE TRUE"));

        if let Some(tenant_id) = self.tenant_id {
            qb.push(" AND chats.tenant_id = ").push_bind(tenant_id);
        }

        if let Some(created_by_id) = self.created_by_id {
            qb.push(" AND chats.created_by_id = ").push_bind(created_by_id);
        }

        if let Some(open) = self.open {
            qb.push(if open {
                " AND chats.closed_at IS NULL"
            } else {
                " AND chats.closed_at IS NOT NULL"
            });
        }

        if let Some(before) = self.before {
            qb.push(" AND chats.created_at < ").push_bind(before);
        }

        if let Some(after) = self.after {
            qb.push(" AND chats.created_at > ").push_bind(after);
        }

        if let Some(cursor) = self.cursor {
            qb.push(" AND chats.id < ").push_bind(cursor);
        }

        qb.push(" ORDER BY chats.id DESC");
        qb.push(" LIMIT ").push_bind((self.limit + 1) as i64);

        let rows = qb
            .build_query_scalar::<sqlx::types::Json<types::chats::Chat>>()
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|sqlx::types::Json(chat)| chat)
            .collect();

        Ok(crate::result(rows, self.limit, |chat| chat.id))
    }
}

impl Default for Query {
    fn default() -> Self {
        Self {
            limit: 10,
            cursor: None,
            tenant_id: None,
            created_by_id: None,
            open: None,
            before: None,
            after: None,
        }
    }
}
