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
    pub external_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<types::actors::Role>>,

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

    pub fn external_id(mut self, value: impl std::fmt::Display) -> Self {
        self.external_id = Some(value.to_string());
        self
    }

    pub fn roles(mut self, value: impl IntoIterator<Item = types::actors::Role>) -> Self {
        self.roles = Some(value.into_iter().collect());
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

    pub async fn exec(&self, pool: &sqlx::PgPool) -> error::Result<QueryResult<types::actors::Actor>> {
        self.validate()?;
        let json = super::project::jsonb_build_object("actors");
        let mut qb = sqlx::QueryBuilder::<sqlx::Postgres>::new(format!("SELECT {json} FROM actors WHERE TRUE"));

        if let Some(tenant_id) = self.tenant_id {
            qb.push(" AND actors.tenant_id = ").push_bind(tenant_id);
        }

        if let Some(external_id) = &self.external_id {
            qb.push(" AND actors.external_id = ").push_bind(external_id);
        }

        if let Some(roles) = &self.roles
            && !roles.is_empty()
        {
            qb.push(" AND actors.role = ANY(")
                .push_bind(roles.iter().map(|role| role.as_str()).collect::<Vec<_>>())
                .push(")");
        }

        if let Some(before) = self.before {
            qb.push(" AND actors.created_at < ").push_bind(before);
        }

        if let Some(after) = self.after {
            qb.push(" AND actors.created_at > ").push_bind(after);
        }

        if let Some(cursor) = self.cursor {
            qb.push(" AND actors.id < ").push_bind(cursor);
        }

        qb.push(" ORDER BY actors.id DESC");
        qb.push(" LIMIT ").push_bind((self.limit + 1) as i64);

        let rows = qb
            .build_query_scalar::<sqlx::types::Json<types::actors::Actor>>()
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|sqlx::types::Json(actor)| actor)
            .collect();

        Ok(crate::result(rows, self.limit, |actor| actor.id))
    }
}

impl Default for Query {
    fn default() -> Self {
        Self {
            limit: 10,
            cursor: None,
            tenant_id: None,
            external_id: None,
            roles: None,
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
