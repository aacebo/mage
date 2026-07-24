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
    pub message_id: Option<uuid::Uuid>,
    pub task_id: Option<uuid::Uuid>,
    #[validate(unique_items)]
    pub types: Option<Vec<String>>,
    #[validate(unique_items)]
    pub labels: Option<Vec<String>>,
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

    pub fn message(mut self, value: uuid::Uuid) -> Self {
        self.message_id = Some(value);
        self
    }

    pub fn task(mut self, value: uuid::Uuid) -> Self {
        self.task_id = Some(value);
        self
    }

    pub fn types(mut self, value: impl IntoIterator<Item = impl std::fmt::Display>) -> Self {
        self.types = Some(value.into_iter().map(|item| item.to_string()).collect());
        self
    }

    pub fn labels(mut self, value: impl IntoIterator<Item = impl std::fmt::Display>) -> Self {
        self.labels = Some(value.into_iter().map(|item| item.to_string()).collect());
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

    pub async fn exec(&self, pool: &sqlx::PgPool) -> error::Result<QueryResult<types::resources::Annotation>> {
        self.validate()?;
        let json = super::project::jsonb_build_object("annotations");
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(format!("SELECT {json} FROM annotations WHERE TRUE"));

        if let Some(message_id) = self.message_id {
            qb.push(" AND annotations.message_id = ").push_bind(message_id);
        }

        if let Some(task_id) = self.task_id {
            qb.push(" AND annotations.task_id = ").push_bind(task_id);
        }

        if let Some(types) = &self.types
            && !types.is_empty()
        {
            qb.push(" AND annotations.type = ANY(").push_bind(types).push(")");
        }

        if let Some(labels) = &self.labels
            && !labels.is_empty()
        {
            qb.push(" AND annotations.label = ANY(").push_bind(labels).push(")");
        }

        if let Some(before) = self.before {
            qb.push(" AND annotations.created_at < ").push_bind(before);
        }

        if let Some(after) = self.after {
            qb.push(" AND annotations.created_at > ").push_bind(after);
        }

        if let Some(cursor) = self.cursor {
            qb.push(" AND annotations.id < ").push_bind(cursor);
        }

        qb.push(" ORDER BY annotations.id DESC");
        qb.push(" LIMIT ").push_bind((self.limit + 1) as i64);

        let rows = qb
            .build_query_scalar::<sqlx::types::Json<types::resources::Annotation>>()
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|sqlx::types::Json(annotation)| annotation)
            .collect();

        Ok(crate::paginate(rows, self.limit, |annotation| annotation.id))
    }
}

impl Default for Query {
    fn default() -> Self {
        Self {
            limit: 10,
            cursor: None,
            message_id: None,
            task_id: None,
            types: None,
            labels: None,
            before: None,
            after: None,
        }
    }
}
