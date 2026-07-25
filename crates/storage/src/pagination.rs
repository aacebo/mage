pub fn result<T>(items: Vec<T>, limit: usize, id: impl Fn(&T) -> uuid::Uuid) -> QueryResult<T> {
    let mut result = QueryResult { next: None, items };

    if result.items.len() > limit {
        result.next = result.items.pop().map(|v| id(&v));
    }

    result
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueryResult<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<uuid::Uuid>,

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
