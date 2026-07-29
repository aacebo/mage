#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Offset {
    pub begin: u32,
    pub end: u32,
}

impl Offset {
    pub fn new(begin: u32, end: u32) -> Self {
        Self { begin, end }
    }
}

impl std::fmt::Display for Offset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.begin, self.end)
    }
}
