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
    pub tenant_id: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub chat_id: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<uuid::Uuid>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub statuses: Option<Vec<mage_types::tasks::TaskStatus>>,

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

    pub fn trace(mut self, value: uuid::Uuid) -> Self {
        self.trace_id = Some(value);
        self
    }

    pub fn parent(mut self, value: uuid::Uuid) -> Self {
        self.parent_id = Some(value);
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

    pub fn agent(mut self, value: uuid::Uuid) -> Self {
        self.agent_id = Some(value);
        self
    }

    pub fn statuses(mut self, value: impl IntoIterator<Item = mage_types::tasks::TaskStatus>) -> Self {
        self.statuses = Some(value.into_iter().collect());
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

    pub async fn exec(&self, pool: &sqlx::PgPool) -> mage_error::Result<QueryResult<mage_types::tasks::Task>> {
        self.validate()?;
        let json = super::project::jsonb_build_object("tasks");
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(format!("SELECT {json} FROM tasks WHERE TRUE"));

        if let Some(tenant_id) = self.tenant_id {
            qb.push(" AND tasks.tenant_id = ").push_bind(tenant_id);
        }

        if let Some(trace_id) = self.trace_id {
            qb.push(" AND tasks.trace_id = ").push_bind(trace_id);
        }

        if let Some(parent_id) = self.parent_id {
            qb.push(" AND tasks.parent_id = ").push_bind(parent_id);
        }

        if let Some(chat_id) = self.chat_id {
            qb.push(" AND tasks.chat_id = ").push_bind(chat_id);
        }

        if let Some(message_id) = self.message_id {
            qb.push(" AND tasks.message_id = ").push_bind(message_id);
        }

        if let Some(agent_id) = self.agent_id {
            qb.push(" AND tasks.agent_id = ").push_bind(agent_id);
        }

        if let Some(statuses) = &self.statuses
            && !statuses.is_empty()
        {
            qb.push(" AND tasks.status = ANY(")
                .push_bind(statuses.iter().map(|status| status.as_str()).collect::<Vec<_>>())
                .push(")");
        }

        if let Some(before) = self.before {
            qb.push(" AND tasks.created_at < ").push_bind(before);
        }

        if let Some(after) = self.after {
            qb.push(" AND tasks.created_at > ").push_bind(after);
        }

        if let Some(cursor) = self.cursor {
            qb.push(" AND tasks.id < ").push_bind(cursor);
        }

        qb.push(" ORDER BY tasks.id DESC");
        qb.push(" LIMIT ").push_bind((self.limit + 1) as i64);

        let rows = qb
            .build_query_scalar::<sqlx::types::Json<mage_types::tasks::Task>>()
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|sqlx::types::Json(task)| task)
            .collect();

        Ok(crate::result(rows, self.limit, |task| task.id))
    }
}

impl Default for Query {
    fn default() -> Self {
        Self {
            limit: 10,
            cursor: None,
            tenant_id: None,
            trace_id: None,
            parent_id: None,
            chat_id: None,
            message_id: None,
            agent_id: None,
            statuses: None,
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
