#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Request<T = serde_json::Value> {
    pub id: uuid::Uuid,
    pub method: String,

    #[serde(flatten)]
    pub params: T,
}
