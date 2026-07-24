use error::Result;
use pgvector::Vector;

pub const EMBEDDING_DIMENSIONS: usize = 384;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchOptions {
    pub limit: u32,
    pub min_similarity: f64,
    pub role: Option<types::actors::Role>,
}

impl SearchOptions {
    pub fn new(limit: u32, min_similarity: f64) -> Result<Self> {
        let options = Self {
            limit,
            min_similarity,
            role: None,
        };

        options.validate()?;
        Ok(options)
    }

    pub fn with_role(mut self, role: types::actors::Role) -> Self {
        self.role = Some(role);
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.limit == 0 {
            return Err(error::bad_request("search limit must be greater than zero"));
        }

        if !self.min_similarity.is_finite() || !(-1.0..=1.0).contains(&self.min_similarity) {
            return Err(error::bad_request(
                "search minimum similarity must be finite and between -1 and 1",
            ));
        }

        Ok(())
    }
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            min_similarity: 0.2,
            role: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult<T> {
    pub entity: T,
    pub similarity: f64,
}

pub fn prepare(embedding: Vec<f32>, options: SearchOptions) -> Result<(Vector, i64, f64)> {
    options.validate()?;

    if embedding.len() != EMBEDDING_DIMENSIONS {
        return Err(error::bad_request(format!(
            "search embedding must have {EMBEDDING_DIMENSIONS} dimensions; received {}",
            embedding.len()
        )));
    }

    if embedding.iter().any(|value| !value.is_finite()) {
        return Err(error::bad_request("search embedding components must be finite"));
    }

    if embedding.iter().all(|value| *value == 0.0) {
        return Err(error::bad_request("search embedding cannot be a zero vector"));
    }

    Ok((Vector::from(embedding), i64::from(options.limit), options.min_similarity))
}
