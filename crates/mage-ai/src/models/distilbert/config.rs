use candle_transformers::models::distilbert;
use mage_error::Result;

use crate::models::Architecture;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub vocab_size: usize,
    pub dim: usize,
    pub n_layers: usize,
    pub n_heads: usize,
    pub hidden_dim: usize,
    pub activation: Activation,
    pub max_position_embeddings: usize,
    pub initializer_range: f64,
    pub pad_token_id: usize,
    #[serde(default)]
    pub model_type: Architecture,
}

impl Config {
    pub fn hidden_size(&self) -> usize {
        self.dim
    }

    pub fn vocab_size(mut self, vocab_size: usize) -> Self {
        self.vocab_size = vocab_size;
        self
    }

    pub fn dim(mut self, dim: usize) -> Self {
        self.dim = dim;
        self
    }

    pub fn n_layers(mut self, n_layers: usize) -> Self {
        self.n_layers = n_layers;
        self
    }

    pub fn n_heads(mut self, n_heads: usize) -> Self {
        self.n_heads = n_heads;
        self
    }

    pub fn hidden_dim(mut self, hidden_dim: usize) -> Self {
        self.hidden_dim = hidden_dim;
        self
    }

    pub fn activation(mut self, activation: Activation) -> Self {
        self.activation = activation;
        self
    }

    pub fn max_position_embeddings(mut self, positions: usize) -> Self {
        self.max_position_embeddings = positions;
        self
    }

    pub fn to_candle(&self) -> Result<distilbert::Config> {
        let json = serde_json::to_value(self)?;
        Ok(serde_json::from_value(json)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Activation {
    Gelu,
    #[serde(rename = "gelu_approximate")]
    GeluApproximate,
    Relu,
}
