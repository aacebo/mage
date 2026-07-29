use candle_core::{Result, Tensor};
use candle_nn::{Embedding, LayerNorm, Linear, VarBuilder, layer_norm, linear};

use super::attention::Attention;
use super::config::Config;
use super::embeddings::LearnedPositionalEmbedding;

#[derive(Debug, Clone)]
pub struct Decoder {
    embed_tokens: Embedding,
    embed_positions: LearnedPositionalEmbedding,
    layernorm_embedding: LayerNorm,
    layers: Vec<DecoderLayer>,
    embed_scale: Option<f64>,
}

impl Decoder {
    pub fn new(cfg: &Config, embed_tokens: &Embedding, vb: VarBuilder) -> Result<Self> {
        let mut layers = Vec::with_capacity(cfg.decoder_layers);
        let vb_l = vb.pp("layers");

        for index in 0..cfg.decoder_layers {
            layers.push(DecoderLayer::new(cfg, vb_l.pp(index))?);
        }

        Ok(Self {
            embed_tokens: embed_tokens.clone(),
            embed_positions: LearnedPositionalEmbedding::new(cfg, vb.pp("embed_positions"))?,
            layernorm_embedding: layer_norm(cfg.d_model, 1e-5, vb.pp("layernorm_embedding"))?,
            layers,
            embed_scale: cfg.scale_embedding.then(|| (cfg.d_model as f64).sqrt()),
        })
    }

    pub fn forward(
        &mut self,
        xs: &Tensor,
        encoder_xs: Option<&Tensor>,
        past_kv_len: usize,
        attn_mask: &Tensor,
    ) -> Result<Tensor> {
        let seq_len = xs.dim(1)?;
        let xs = xs.apply(&self.embed_tokens)?;
        let xs = match self.embed_scale {
            None => xs,
            Some(scale) => (xs * scale)?,
        };

        let positions = self
            .embed_positions
            .forward(seq_len, past_kv_len, xs.device())?
            .unsqueeze(0)?;
        let mut xs = xs.broadcast_add(&positions)?.apply(&self.layernorm_embedding)?;

        for layer in self.layers.iter_mut() {
            xs = layer.forward(&xs, encoder_xs, attn_mask)?;
        }

        Ok(xs)
    }

    pub fn reset_kv_cache(&mut self) {
        for layer in self.layers.iter_mut() {
            layer.reset_kv_cache();
        }
    }

    pub fn reorder_kv_cache(&mut self, beams: &Tensor) -> Result<()> {
        for layer in self.layers.iter_mut() {
            layer.reorder_kv_cache(beams)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DecoderLayer {
    self_attn: Attention,
    self_attn_layer_norm: LayerNorm,
    activation_fn: candle_nn::Activation,
    encoder_attn: Attention,
    encoder_attn_layer_norm: LayerNorm,
    fc1: Linear,
    fc2: Linear,
    final_layer_norm: LayerNorm,
}

impl DecoderLayer {
    pub fn new(cfg: &Config, vb: VarBuilder) -> Result<Self> {
        Ok(Self {
            self_attn: Attention::new(cfg, true, vb.pp("self_attn"))?,
            self_attn_layer_norm: layer_norm(cfg.d_model, 1e-5, vb.pp("self_attn_layer_norm"))?,
            activation_fn: cfg.activation_function,
            encoder_attn: Attention::new(cfg, true, vb.pp("encoder_attn"))?,
            encoder_attn_layer_norm: layer_norm(cfg.d_model, 1e-5, vb.pp("encoder_attn_layer_norm"))?,
            fc1: linear(cfg.d_model, cfg.decoder_ffn_dim, vb.pp("fc1"))?,
            fc2: linear(cfg.decoder_ffn_dim, cfg.d_model, vb.pp("fc2"))?,
            final_layer_norm: layer_norm(cfg.d_model, 1e-5, vb.pp("final_layer_norm"))?,
        })
    }

    pub fn forward(&mut self, xs: &Tensor, encoder_xs: Option<&Tensor>, attn_mask: &Tensor) -> Result<Tensor> {
        let residual = xs;
        let xs = (self.self_attn.forward(xs, None, Some(attn_mask))? + residual)?.apply(&self.self_attn_layer_norm)?;
        let xs = match encoder_xs {
            None => xs,
            Some(encoder_xs) => {
                let residual = &xs;
                let xs = self.encoder_attn.forward(&xs, Some(encoder_xs), None)?;
                (residual + xs)?.apply(&self.encoder_attn_layer_norm)?
            }
        };

        let residual = &xs;
        let xs = xs.apply(&self.fc1)?.apply(&self.activation_fn)?.apply(&self.fc2)?;
        (xs + residual)?.apply(&self.final_layer_norm)
    }

    pub fn reset_kv_cache(&mut self) {
        self.self_attn.reset_kv_cache();
        self.encoder_attn.reset_kv_cache();
    }

    pub fn reorder_kv_cache(&mut self, beams: &Tensor) -> Result<()> {
        self.self_attn.reorder_kv_cache(beams)
    }
}
