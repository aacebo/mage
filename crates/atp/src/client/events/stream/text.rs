#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamTextEvent {
    pub stream_id: String,
    pub sequence: usize,
    pub text: String,
    pub span: Option<TextSpan>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
}
