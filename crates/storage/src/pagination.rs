#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueryResult<T> {
    pub next: Option<uuid::Uuid>,
    pub items: Vec<T>,
}

pub(crate) fn paginate<T>(items: Vec<T>, limit: usize, id: impl Fn(&T) -> uuid::Uuid) -> QueryResult<T> {
    let mut result = QueryResult { next: None, items };

    if result.items.len() > limit {
        result.items.pop();
        result.next = result.items.last().map(id);
    }

    result
}

#[derive(Debug, Default, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    Asc,
    #[default]
    Desc,
}
