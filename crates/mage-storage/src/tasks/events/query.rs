use serde_valid::Validate;

use crate::{Order, QueryResult};

pub fn new(task_id: uuid::Uuid) -> Query {
    Query {
        limit: default::limit(),
        cursor: None,
        order: Order::Asc,
        tenant_id: None,
        task_id,
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct Query {
    #[validate(minimum = 1)]
    #[validate(maximum = 100)]
    #[serde(default = "default::limit")]
    pub limit: usize,

    #[validate(minimum = 1)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<u64>,

    #[serde(default = "default::order")]
    pub order: Order,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<uuid::Uuid>,

    pub task_id: uuid::Uuid,
}

impl Query {
    pub fn limit(mut self, value: usize) -> Self {
        self.limit = value;
        self
    }

    pub fn cursor(mut self, value: u64) -> Self {
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

    pub async fn exec(&self, pool: &sqlx::PgPool) -> mage_error::Result<QueryResult<mage_types::tasks::TaskEvent, u64>> {
        self.validate()?;
        let cursor = self
            .cursor
            .map(i64::try_from)
            .transpose()
            .map_err(|_| mage_error::bad_request("task event sequence exceeds PostgreSQL BIGINT"))?;

        let json = super::project::jsonb_build_object("task_events");
        let mut qb =
            sqlx::QueryBuilder::<sqlx::Postgres>::new(format!("SELECT {json} FROM task_events WHERE task_events.task_id = "));
        qb.push_bind(self.task_id);

        if let Some(tenant_id) = self.tenant_id {
            qb.push(" AND task_events.tenant_id = ").push_bind(tenant_id);
        }

        if let Some(cursor) = cursor {
            qb.push(match self.order {
                Order::Asc => " AND task_events.sequence > ",
                Order::Desc => " AND task_events.sequence < ",
            })
            .push_bind(cursor);
        }

        qb.push(match self.order {
            Order::Asc => " ORDER BY task_events.sequence",
            Order::Desc => " ORDER BY task_events.sequence DESC",
        });
        qb.push(" LIMIT ").push_bind((self.limit + 1) as i64);

        let rows = qb
            .build_query_scalar::<sqlx::types::Json<mage_types::tasks::TaskEvent>>()
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|sqlx::types::Json(event)| event)
            .collect();

        Ok(crate::result(rows, self.limit, |event| event.sequence))
    }
}

mod default {
    pub fn limit() -> usize {
        10
    }

    pub fn order() -> crate::Order {
        crate::Order::Asc
    }
}
