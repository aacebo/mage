use std::collections::HashMap;

use candle_transformers::models::bert;
use mage_error::Result;

use crate::models::Architecture;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub vocab_size: usize,
    pub hidden_size: usize,
    pub num_hidden_layers: usize,
    pub num_attention_heads: usize,
    pub intermediate_size: usize,
    pub hidden_act: Activation,
    pub hidden_dropout_prob: f64,
    pub max_position_embeddings: usize,
    pub type_vocab_size: usize,
    pub initializer_range: f64,
    pub layer_norm_eps: f64,
    pub pad_token_id: usize,
    #[serde(default)]
    pub classifier_dropout: Option<f64>,
    #[serde(default)]
    pub model_type: Architecture,
    #[serde(default)]
    pub id2label: HashMap<String, String>,
}

impl From<&Config> for bert::Config {
    fn from(config: &Config) -> Self {
        Self {
            vocab_size: config.vocab_size,
            hidden_size: config.hidden_size,
            num_hidden_layers: config.num_hidden_layers,
            num_attention_heads: config.num_attention_heads,
            intermediate_size: config.intermediate_size,
            hidden_act: config.hidden_act.into(),
            hidden_dropout_prob: config.hidden_dropout_prob,
            max_position_embeddings: config.max_position_embeddings,
            type_vocab_size: config.type_vocab_size,
            initializer_range: config.initializer_range,
            layer_norm_eps: config.layer_norm_eps,
            pad_token_id: config.pad_token_id,
            position_embedding_type: bert::PositionEmbeddingType::Absolute,
            use_cache: false,
            classifier_dropout: config.classifier_dropout,
            model_type: Some(config.model_type.to_string()),
        }
    }
}

impl Config {
    pub fn vocab_size(mut self, vocab_size: usize) -> Self {
        self.vocab_size = vocab_size;
        self
    }

    pub fn hidden_size(mut self, hidden_size: usize) -> Self {
        self.hidden_size = hidden_size;
        self
    }

    pub fn num_hidden_layers(mut self, layers: usize) -> Self {
        self.num_hidden_layers = layers;
        self
    }

    pub fn num_attention_heads(mut self, heads: usize) -> Self {
        self.num_attention_heads = heads;
        self
    }

    pub fn intermediate_size(mut self, size: usize) -> Self {
        self.intermediate_size = size;
        self
    }

    pub fn hidden_act(mut self, act: Activation) -> Self {
        self.hidden_act = act;
        self
    }

    pub fn max_position_embeddings(mut self, positions: usize) -> Self {
        self.max_position_embeddings = positions;
        self
    }

    pub fn id2label(mut self, id2label: HashMap<String, String>) -> Self {
        self.id2label = id2label;
        self
    }

    /// A label map is what a classification head announces: a checkpoint carrying `id2label` can
    /// token-classify, one without it is an embedder.
    pub fn has_labels(&self) -> bool {
        !self.id2label.is_empty()
    }

    /// `id2label` is keyed by stringified index; flatten it into a dense, ordered Vec.
    pub fn labels(&self) -> Result<Vec<String>> {
        let mut labels = vec![String::new(); self.id2label.len()];

        for (index, label) in &self.id2label {
            let index: usize = index
                .parse()
                .map_err(|_| mage_error::ai(format!("invalid label index: {index}")))?;

            let slot = labels
                .get_mut(index)
                .ok_or_else(|| mage_error::ai(format!("label index out of range: {index}")))?;

            *slot = label.clone();
        }

        Ok(labels)
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

impl From<Activation> for bert::HiddenAct {
    fn from(activation: Activation) -> Self {
        match activation {
            Activation::Gelu => Self::Gelu,
            Activation::GeluApproximate => Self::GeluApproximate,
            Activation::Relu => Self::Relu,
        }
    }
}
