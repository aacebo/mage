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
    pub cursor: Option<uuid::Uuid>,
    pub trace_id: Option<uuid::Uuid>,
    pub tenant_id: Option<uuid::Uuid>,
    pub task_id: Option<uuid::Uuid>,
    #[validate(unique_items)]
    pub levels: Option<Vec<types::logs::Level>>,
    pub source: Option<String>,
    pub created_by_id: Option<uuid::Uuid>,
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

    pub fn trace(mut self, value: uuid::Uuid) -> Self {
        self.trace_id = Some(value);
        self
    }

    pub fn tenant(mut self, value: uuid::Uuid) -> Self {
        self.tenant_id = Some(value);
        self
    }

    pub fn task(mut self, value: uuid::Uuid) -> Self {
        self.task_id = Some(value);
        self
    }

    pub fn levels(mut self, value: impl IntoIterator<Item = types::logs::Level>) -> Self {
        self.levels = Some(value.into_iter().collect());
        self
    }

    pub fn source(mut self, value: impl std::fmt::Display) -> Self {
        self.source = Some(value.to_string());
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

    pub async fn exec(&self, pool: &sqlx::PgPool) -> error::Result<QueryResult<types::logs::Log>> {
        self.validate()?;
        let json = super::project::jsonb_build_object("logs");
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(format!(
            r#"
            SELECT {json}
            FROM logs
            WHERE TRUE
            "#,
        ));

        if let Some(tenant_id) = self.tenant_id {
            qb.push(" AND logs.tenant_id = ").push_bind(tenant_id);
        }

        if let Some(trace_id) = self.trace_id {
            qb.push(" AND logs.trace_id = ").push_bind(trace_id);
        }

        if let Some(task_id) = self.task_id {
            qb.push(" AND logs.task_id = ").push_bind(task_id);
        }

        if let Some(levels) = &self.levels
            && !levels.is_empty()
        {
            qb.push(" AND logs.level = ANY(")
                .push_bind(levels.iter().map(|lvl| lvl.as_str()).collect::<Vec<_>>())
                .push(")");
        }

        if let Some(source) = &self.source {
            qb.push(" AND logs.source = ").push_bind(source);
        }

        if let Some(created_by_id) = self.created_by_id {
            qb.push(" AND logs.created_by_id = ").push_bind(created_by_id);
        }

        if let Some(before) = self.before {
            qb.push(" AND logs.created_at < ").push_bind(before);
        }

        if let Some(after) = self.after {
            qb.push(" AND logs.created_at > ").push_bind(after);
        }

        if let Some(cursor) = &self.cursor {
            qb.push(" AND logs.id < ").push_bind(cursor);
        }

        qb.push(" ORDER BY logs.id DESC");
        qb.push(" LIMIT ").push_bind((self.limit + 1) as i64);

        let rows = qb
            .build_query_scalar::<sqlx::types::Json<types::logs::Log>>()
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|sqlx::types::Json(log)| log)
            .collect::<Vec<_>>();

        let mut result = QueryResult { next: None, items: rows };

        if result.items.len() > self.limit {
            result.items.pop();
            result.next = result.items.last().map(|v| v.id);
        }

        Ok(result)
    }
}

impl Default for Query {
    fn default() -> Self {
        Self {
            limit: 10,
            cursor: None,
            trace_id: None,
            tenant_id: None,
            task_id: None,
            levels: None,
            source: None,
            created_by_id: None,
            before: None,
            after: None,
        }
    }
}
