pub fn result<T, C>(items: Vec<T>, limit: usize, cursor: impl Fn(&T) -> C) -> QueryResult<T, C> {
    let mut result = QueryResult { next: None, items };

    if result.items.len() > limit {
        result.items.pop();
        result.next = result.items.last().map(cursor);
    }

    result
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueryResult<T, C = uuid::Uuid> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<C>,

    #[serde(default)]
    pub items: Vec<T>,
}

#[derive(Debug, Default, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Order {
    Asc,
    #[default]
    Desc,
}
