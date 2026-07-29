#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RoutingPolicy {
    pub candidate_limit: u32,
    pub min_confidence: f64,
    pub ambiguity_margin: f64,
}

impl RoutingPolicy {
    const SCORE_EPSILON: f64 = 1e-12;

    pub fn new(candidate_limit: u32, min_confidence: f64, ambiguity_margin: f64) -> mage_error::Result<Self> {
        let policy = Self {
            candidate_limit,
            min_confidence,
            ambiguity_margin,
        };

        policy.validate()?;
        Ok(policy)
    }

    pub fn validate(&self) -> mage_error::Result<()> {
        if self.candidate_limit < 2 {
            return Err(mage_error::config("routing candidate limit must be at least 2"));
        }

        if !self.min_confidence.is_finite() || !(-1.0..=1.0).contains(&self.min_confidence) {
            return Err(mage_error::config(
                "routing minimum confidence must be finite and between -1 and 1",
            ));
        }

        if !self.ambiguity_margin.is_finite() || !(0.0..=2.0).contains(&self.ambiguity_margin) {
            return Err(mage_error::config(
                "routing ambiguity margin must be finite and between 0 and 2",
            ));
        }

        Ok(())
    }

    pub fn decide<T: Clone>(&self, mut candidates: Vec<mage_storage::SearchResult<T>>) -> RoutingDecision<T> {
        candidates.retain(|candidate| candidate.similarity.is_finite());
        candidates.sort_by(|a, b| b.similarity.total_cmp(&a.similarity));
        candidates.truncate(self.candidate_limit as usize);

        let Some(top) = candidates.first() else {
            return RoutingDecision::NoRoute {
                reason: NoRouteReason::NoCandidates,
                candidates,
            };
        };

        if top.similarity < self.min_confidence {
            return RoutingDecision::NoRoute {
                reason: NoRouteReason::LowConfidence,
                candidates,
            };
        }

        let top_similarity = top.similarity;
        let mut agents = vec![top.clone()];
        agents.extend(
            candidates
                .iter()
                .skip(1)
                .filter(|candidate| {
                    candidate.similarity >= self.min_confidence
                        && top_similarity - candidate.similarity + Self::SCORE_EPSILON < self.ambiguity_margin
                })
                .cloned(),
        );

        RoutingDecision::Selected { agents, candidates }
    }
}

impl Default for RoutingPolicy {
    fn default() -> Self {
        Self {
            candidate_limit: 5,
            min_confidence: 0.20,
            ambiguity_margin: 0.05,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoRouteReason {
    NoCandidates,
    LowConfidence,
}

impl NoRouteReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoCandidates => "no_candidates",
            Self::LowConfidence => "low_confidence",
        }
    }
}

#[derive(Debug, Clone)]
pub enum RoutingDecision<T> {
    Selected {
        agents: Vec<mage_storage::SearchResult<T>>,
        candidates: Vec<mage_storage::SearchResult<T>>,
    },
    NoRoute {
        reason: NoRouteReason,
        candidates: Vec<mage_storage::SearchResult<T>>,
    },
}
