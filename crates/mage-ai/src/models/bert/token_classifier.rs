use candle_core::Tensor;
use candle_nn::{Linear, Module, VarBuilder, ops};
use mage_error::Result;

use super::config::Config;
use super::model::Bert;
use crate::models::{Context, Forward, TokenClassify, Word};
use crate::tasks::{aggregation, bioes};
use crate::types::Entity;

pub struct TokenClassifier {
    bert: Bert,
    classifier: Linear,
    labels: Vec<String>,
}

impl TokenClassifier {
    pub fn new(vars: VarBuilder, config: &Config) -> Result<Self> {
        let labels = config.labels()?;
        let classifier = candle_nn::linear(config.hidden_size, labels.len(), vars.pp("classifier")).map_err(mage_error::ai)?;

        Ok(Self {
            bert: Bert::new(vars, config)?,
            classifier,
            labels,
        })
    }

    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// Per-token label probabilities, shaped `(batch, seq, labels)`.
    pub fn forward(&self, ids: &Tensor, mask: &Tensor) -> Result<Tensor> {
        let hidden = self.bert.forward(ids, mask)?;

        self.classifier
            .forward(&hidden)
            .and_then(|logits| ops::softmax(&logits, 2))
            .map_err(mage_error::ai)
    }
}

impl Forward for TokenClassifier {
    type Input = (Tensor, Tensor);
    type Output = Tensor;

    fn forward(&self, (ids, mask): Self::Input) -> Result<Self::Output> {
        self.forward(&ids, &mask)
    }
}

impl TokenClassify for TokenClassifier {
    /// IOB1 decoding: CoNLL-03 entities may start with `I-`, and `B-` only splits adjacent
    /// same-type entities.
    fn entities(&self, cx: &Context, text: &[&str]) -> Result<Vec<Vec<Entity>>> {
        text.iter()
            .map(|text| Ok(aggregation::entities(self.words(cx, text)?, text)))
            .collect()
    }

    /// BIOES decoding plus a score filter -- a different decode of the same labelled words.
    fn pii(&self, cx: &Context, text: &[&str], min_score: f64) -> Result<Vec<Vec<Entity>>> {
        text.iter()
            .map(|text| Ok(bioes::entities(self.words(cx, text)?, text, min_score)))
            .collect()
    }
}

impl TokenClassifier {
    /// Sub-word pieces merged into whole words, each carrying the label its tokens voted for.
    /// Both decodes above start here.
    fn words(&self, cx: &Context, text: &str) -> Result<Vec<Word>> {
        let encoding = cx.encode_one(text)?;
        let ids = encoding.get_ids();

        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let shape = (1, ids.len());
        let input = Tensor::from_slice(ids, shape, cx.device()).map_err(mage_error::ai)?;
        let mask = Tensor::from_slice(encoding.get_attention_mask(), shape, cx.device()).map_err(mage_error::ai)?;
        let probs = self
            .forward(&input, &mask)?
            .squeeze(0)
            .and_then(|probs| probs.to_vec2::<f32>())
            .map_err(mage_error::ai)?;

        aggregation::words(&probs, &encoding, self.labels())
    }
}
