use serde_valid::Validate;

use crate::{Order, QueryResult};

pub fn new() -> Query {
    Query::default()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct Query {
    #[validate(minimum = 1)]
    #[validate(maximum = 100)]
    pub limit: usize,
    pub cursor: Option<uuid::Uuid>,
    pub order: Order,
    pub tenant_id: Option<uuid::Uuid>,
    pub trace_id: Option<uuid::Uuid>,
    pub actor_id: Option<uuid::Uuid>,
    pub chat_id: Option<uuid::Uuid>,
    pub message_id: Option<uuid::Uuid>,
    pub task_id: Option<uuid::Uuid>,
    #[validate(unique_items)]
    pub keys: Option<Vec<String>>,
    pub before: Option<chrono::DateTime<chrono::Utc>>,
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

    pub fn ascending(mut self) -> Self {
        self.order = Order::Asc;
        self
    }

    pub fn descending(mut self) -> Self {
        self.order = Order::Desc;
        self
    }

    pub fn tenant(mut self, value: uuid::Uuid) -> Self {
        self.tenant_id = Some(value);
        self
    }

    pub fn trace(mut self, value: uuid::Uuid) -> Self {
        self.trace_id = Some(value);
        self
    }

    pub fn actor(mut self, value: uuid::Uuid) -> Self {
        self.actor_id = Some(value);
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

    pub fn keys(mut self, value: impl IntoIterator<Item = impl std::fmt::Display>) -> Self {
        self.keys = Some(value.into_iter().map(|item| item.to_string()).collect());
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

    pub async fn exec(&self, pool: &sqlx::PgPool) -> error::Result<QueryResult<types::events::Event>> {
        self.validate()?;
        let json = super::project::jsonb_build_object("events");
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(format!("SELECT {json} FROM events WHERE TRUE"));

        if let Some(tenant_id) = self.tenant_id {
            qb.push(" AND events.tenant_id = ").push_bind(tenant_id);
        }

        if let Some(trace_id) = self.trace_id {
            qb.push(" AND events.trace_id = ").push_bind(trace_id);
        }

        if let Some(actor_id) = self.actor_id {
            qb.push(" AND events.actor_id = ").push_bind(actor_id);
        }

        if let Some(chat_id) = self.chat_id {
            qb.push(" AND events.chat_id = ").push_bind(chat_id);
        }

        if let Some(message_id) = self.message_id {
            qb.push(" AND events.message_id = ").push_bind(message_id);
        }

        if let Some(task_id) = self.task_id {
            qb.push(" AND events.task_id = ").push_bind(task_id);
        }

        if let Some(keys) = &self.keys
            && !keys.is_empty()
        {
            qb.push(" AND events.key = ANY(").push_bind(keys).push(")");
        }

        if let Some(before) = self.before {
            qb.push(" AND events.created_at < ").push_bind(before);
        }

        if let Some(after) = self.after {
            qb.push(" AND events.created_at > ").push_bind(after);
        }

        if let Some(cursor) = self.cursor {
            qb.push(match self.order {
                Order::Asc => " AND events.id > ",
                Order::Desc => " AND events.id < ",
            })
            .push_bind(cursor);
        }

        qb.push(match self.order {
            Order::Asc => " ORDER BY events.id",
            Order::Desc => " ORDER BY events.id DESC",
        });
        qb.push(" LIMIT ").push_bind((self.limit + 1) as i64);

        let rows = qb
            .build_query_scalar::<sqlx::types::Json<types::events::Event>>()
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|sqlx::types::Json(event)| event)
            .collect();

        Ok(crate::paginate(rows, self.limit, |event| event.id))
    }
}

impl Default for Query {
    fn default() -> Self {
        Self {
            limit: 10,
            cursor: None,
            order: Order::Desc,
            tenant_id: None,
            trace_id: None,
            actor_id: None,
            chat_id: None,
            message_id: None,
            task_id: None,
            keys: None,
            before: None,
            after: None,
        }
    }
}
