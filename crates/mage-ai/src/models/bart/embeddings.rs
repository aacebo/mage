use candle_core::{Result, Tensor};
use candle_nn::{Embedding, VarBuilder, embedding};

use super::config::Config;

/// BART's learned positional embeddings are offset by 2, a legacy artifact of `pad_token_id = 1`.
const POSITION_OFFSET: u32 = 2;

#[derive(Debug, Clone)]
pub struct LearnedPositionalEmbedding {
    emb: Embedding,
}

impl LearnedPositionalEmbedding {
    pub fn new(cfg: &Config, vb: VarBuilder) -> Result<Self> {
        let positions = cfg.max_position_embeddings + POSITION_OFFSET as usize;

        Ok(Self {
            emb: embedding(positions, cfg.d_model, vb)?,
        })
    }

    pub fn forward(&self, seq_len: usize, past_kv_len: usize, device: &candle_core::Device) -> Result<Tensor> {
        let start = past_kv_len as u32 + POSITION_OFFSET;
        Tensor::arange(start, start + seq_len as u32, device)?.apply(&self.emb)
    }
}
