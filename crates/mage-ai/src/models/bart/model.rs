use candle_core::{DType, Result, Tensor};
use candle_nn::{Linear, VarBuilder, embedding};

use super::config::Config;
use super::decoder::Decoder;
use super::encoder::Encoder;

#[derive(Debug, Clone)]
pub struct Bart {
    encoder: Encoder,
    decoder: Decoder,
    lm_head: Linear,
    final_logits_bias: Option<Tensor>,
}

impl Bart {
    pub fn new(cfg: &Config, vb: VarBuilder) -> Result<Self> {
        let vb_m = vb.pp("model");
        let embed_tokens = embedding(cfg.vocab_size, cfg.d_model, vb_m.pp("decoder").pp("embed_tokens"))?;

        Ok(Self {
            encoder: Encoder::new(cfg, &embed_tokens, vb_m.pp("encoder"))?,
            decoder: Decoder::new(cfg, &embed_tokens, vb_m.pp("decoder"))?,
            lm_head: Linear::new(embed_tokens.embeddings().clone(), None),
            final_logits_bias: vb.get((1, cfg.vocab_size), "final_logits_bias").ok(),
        })
    }

    pub fn encode(&mut self, xs: &Tensor) -> Result<Tensor> {
        self.encoder.forward(xs)
    }

    pub fn decode(&mut self, xs: &Tensor, encoder_xs: &Tensor, past_kv_len: usize) -> Result<Tensor> {
        let seq_len = xs.dim(1)?;
        let mask: Vec<f32> = (0..seq_len)
            .flat_map(|i| (0..seq_len).map(move |j| if j > i { f32::NEG_INFINITY } else { 0f32 }))
            .collect();
        let mask = Tensor::from_vec(mask, (seq_len, seq_len), xs.device())?.to_dtype(DType::F32)?;
        let logits = self
            .decoder
            .forward(xs, Some(encoder_xs), past_kv_len, &mask)?
            .apply(&self.lm_head)?;

        match &self.final_logits_bias {
            None => Ok(logits),
            Some(bias) => logits.broadcast_add(bias),
        }
    }

    pub fn reset_kv_cache(&mut self) {
        self.decoder.reset_kv_cache();
    }

    pub fn reorder_kv_cache(&mut self, beams: &Tensor) -> Result<()> {
        self.decoder.reorder_kv_cache(beams)
    }
}
