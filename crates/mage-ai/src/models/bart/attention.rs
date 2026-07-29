use candle_core::{Result, Tensor};
use candle_nn::{Linear, VarBuilder, linear};

use super::config::Config;

#[derive(Debug, Clone)]
pub struct Attention {
    q_proj: Linear,
    k_proj: Linear,
    v_proj: Linear,
    out_proj: Linear,
    scaling: f64,
    num_heads: usize,
    head_dim: usize,
    kv_cache: Option<(Tensor, Tensor)>,
    cross_cache: Option<(Tensor, Tensor)>,
    is_decoder: bool,
}

impl Attention {
    pub fn new(cfg: &Config, is_decoder: bool, vb: VarBuilder) -> Result<Self> {
        let num_heads = if is_decoder {
            cfg.decoder_attention_heads
        } else {
            cfg.encoder_attention_heads
        };

        let embed_dim = cfg.d_model;
        let head_dim = embed_dim / num_heads;

        Ok(Self {
            q_proj: linear(embed_dim, embed_dim, vb.pp("q_proj"))?,
            k_proj: linear(embed_dim, embed_dim, vb.pp("k_proj"))?,
            v_proj: linear(embed_dim, embed_dim, vb.pp("v_proj"))?,
            out_proj: linear(embed_dim, embed_dim, vb.pp("out_proj"))?,
            scaling: (head_dim as f64).powf(-0.5),
            num_heads,
            head_dim,
            kv_cache: None,
            cross_cache: None,
            is_decoder,
        })
    }

    pub fn forward(&mut self, xs: &Tensor, kv_states: Option<&Tensor>, attn_mask: Option<&Tensor>) -> Result<Tensor> {
        let (b_sz, tgt_len, _) = xs.dims3()?;
        let query_states = (xs.apply(&self.q_proj)? * self.scaling)?;
        let (key_states, value_states) = match kv_states {
            Some(kv_states) => match &self.cross_cache {
                Some((key, value)) => (key.clone(), value.clone()),
                None => {
                    let key = self.shape(&kv_states.apply(&self.k_proj)?, b_sz)?;
                    let value = self.shape(&kv_states.apply(&self.v_proj)?, b_sz)?;

                    self.cross_cache = Some((key.clone(), value.clone()));
                    (key, value)
                }
            },
            None => {
                let key_states = self.shape(&xs.apply(&self.k_proj)?, b_sz)?;
                let value_states = self.shape(&xs.apply(&self.v_proj)?, b_sz)?;

                if !self.is_decoder {
                    (key_states, value_states)
                } else {
                    let kv_states = match &self.kv_cache {
                        None => (key_states, value_states),
                        Some((prev_key, prev_value)) => (
                            Tensor::cat(&[prev_key, &key_states], 2)?,
                            Tensor::cat(&[prev_value, &value_states], 2)?,
                        ),
                    };

                    self.kv_cache = Some(kv_states.clone());
                    kv_states
                }
            }
        };

        let proj_shape = (b_sz * self.num_heads, (), self.head_dim);
        let query_states = self.shape(&query_states, b_sz)?.reshape(proj_shape)?;
        let key_states = key_states.reshape(proj_shape)?;
        let value_states = value_states.reshape(proj_shape)?;
        let attn_weights = query_states.matmul(&key_states.transpose(1, 2)?)?;
        let attn_weights = match attn_mask {
            None => attn_weights,
            Some(attn_mask) => attn_weights.broadcast_add(attn_mask)?,
        };

        candle_nn::ops::softmax_last_dim(&attn_weights)?
            .matmul(&value_states)?
            .reshape((b_sz, self.num_heads, tgt_len, self.head_dim))?
            .transpose(1, 2)?
            .reshape((b_sz, tgt_len, self.head_dim * self.num_heads))?
            .apply(&self.out_proj)
    }

    pub fn reset_kv_cache(&mut self) {
        self.kv_cache = None;
        self.cross_cache = None;
    }

    pub fn reorder_kv_cache(&mut self, beams: &Tensor) -> Result<()> {
        if let Some((key, value)) = &self.kv_cache {
            self.kv_cache = Some((
                key.index_select(beams, 0)?.contiguous()?,
                value.index_select(beams, 0)?.contiguous()?,
            ));
        }

        Ok(())
    }

    fn shape(&self, tensor: &Tensor, bsz: usize) -> Result<Tensor> {
        tensor
            .reshape((bsz, (), self.num_heads, self.head_dim))?
            .transpose(1, 2)?
            .contiguous()
    }
}
