use candle_core::{DType, Tensor};
use candle_nn::VarBuilder;
use error::Result;

use super::config::Config;
use super::model::Bert;
use crate::models::{Context, Embed, Forward};

pub struct Embedder {
    bert: Bert,
}

impl Embedder {
    pub fn new(vars: VarBuilder, config: &Config) -> Result<Self> {
        Ok(Self {
            bert: Bert::new(vars, config)?,
        })
    }

    /// Sentence vectors: mean-pool the token states over the mask, then L2 normalize.
    pub fn forward(&self, ids: &Tensor, mask: &Tensor) -> Result<Tensor> {
        let hidden = self.bert.forward(ids, mask)?;
        normalize(&pool(&hidden, mask)?)
    }
}

fn pool(hidden: &Tensor, mask: &Tensor) -> Result<Tensor> {
    let mask = mask
        .to_dtype(DType::F32)
        .and_then(|mask| mask.unsqueeze(2))
        .map_err(error::ai)?;

    let summed = hidden.broadcast_mul(&mask).and_then(|v| v.sum(1)).map_err(error::ai)?;
    let counts = mask.sum(1).map_err(error::ai)?;
    summed.broadcast_div(&counts).map_err(error::ai)
}

fn normalize(v: &Tensor) -> Result<Tensor> {
    let norm = v
        .sqr()
        .and_then(|v| v.sum_keepdim(1))
        .and_then(|v| v.sqrt())
        .map_err(error::ai)?;

    v.broadcast_div(&norm).map_err(error::ai)
}

impl Forward for Embedder {
    type Input = (Tensor, Tensor);
    type Output = Tensor;

    fn forward(&self, (ids, mask): Self::Input) -> Result<Self::Output> {
        self.forward(&ids, &mask)
    }
}

impl Embed for Embedder {
    fn embed(&self, cx: &Context, text: &[&str]) -> Result<Vec<Vec<f32>>> {
        if text.is_empty() {
            return Ok(Vec::new());
        }

        let batch = cx.encode(text)?;
        self.forward(&batch.ids, &batch.mask)?.to_vec2::<f32>().map_err(error::ai)
    }
}
