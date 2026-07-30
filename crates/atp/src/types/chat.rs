#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Chat {
    pub id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}
