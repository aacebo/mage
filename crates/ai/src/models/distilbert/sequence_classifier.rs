use candle_core::{IndexOp, Tensor};
use candle_nn::{Linear, Module, VarBuilder, ops};
use error::Result;

use super::config::Config;
use super::model::DistilBert;
use crate::models::{Classify, Context, Forward, Label};

const LABELS: usize = 2;

pub struct SequenceClassifier {
    distilbert: DistilBert,
    pre_classifier: Linear,
    classifier: Linear,
}

impl SequenceClassifier {
    pub fn new(vars: VarBuilder, config: &Config) -> Result<Self> {
        let hidden = config.hidden_size();

        Ok(Self {
            distilbert: DistilBert::new(vars.clone(), config)?,
            pre_classifier: candle_nn::linear(hidden, hidden, vars.pp("pre_classifier")).map_err(error::ai)?,
            classifier: candle_nn::linear(hidden, LABELS, vars.pp("classifier")).map_err(error::ai)?,
        })
    }

    pub fn forward(&self, ids: &Tensor, padding: &Tensor) -> Result<Vec<Vec<f32>>> {
        let hidden = self.distilbert.forward(ids, padding)?;
        let pooled = hidden.i((.., 0)).map_err(error::ai)?;
        self.pre_classifier
            .forward(&pooled)
            .and_then(|v| v.relu())
            .and_then(|v| self.classifier.forward(&v))
            .and_then(|logits| ops::softmax(&logits, 1))
            .and_then(|probs| probs.to_vec2::<f32>())
            .map_err(error::ai)
    }
}

impl Forward for SequenceClassifier {
    type Input = (Tensor, Tensor);
    type Output = Vec<Vec<f32>>;

    fn forward(&self, (ids, padding): Self::Input) -> Result<Self::Output> {
        self.forward(&ids, &padding)
    }
}

impl Classify for SequenceClassifier {
    fn classify(&self, cx: &Context, text: &[&str]) -> Result<Vec<Label>> {
        if text.is_empty() {
            return Ok(Vec::new());
        }

        let batch = cx.encode(text)?;
        let probs = self.forward(&batch.ids, &batch.padding()?)?;

        Ok(probs
            .into_iter()
            .map(|row| {
                // SST-2 label order: 0 = NEGATIVE, 1 = POSITIVE. The head emits the label; reading
                // it as a sentiment polarity is the task's task, not the model's.
                let (negative, positive) = (row[0], row[1]);

                match positive >= negative {
                    true => Label {
                        label: "positive".to_string(),
                        score: positive as f64,
                    },
                    false => Label {
                        label: "negative".to_string(),
                        score: negative as f64,
                    },
                }
            })
            .collect())
    }
}
