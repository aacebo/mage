#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Polarity {
    Positive,
    Negative,
}

impl Polarity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Positive => "positive",
            Self::Negative => "negative",
        }
    }

    /// Sentiment is a binary reading of a general classification label. SST-2 emits `NEGATIVE` /
    /// `POSITIVE`; a hosted model answers in whatever case it likes. Anything that is not
    /// recognisably negative reads as positive, matching the old `positive >= negative` rule.
    pub fn from_label(label: &str) -> Self {
        match label.trim().to_lowercase().as_str() {
            "negative" | "neg" | "label_0" => Self::Negative,
            _ => Self::Positive,
        }
    }
}

impl std::fmt::Display for Polarity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Sentiment {
    pub polarity: Polarity,
    pub score: f64,
}

impl std::fmt::Display for Sentiment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({:.3})", self.polarity, self.score)
    }
}
