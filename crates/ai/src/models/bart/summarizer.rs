use std::sync::Mutex;

use candle_core::Tensor;
use candle_nn::VarBuilder;
use error::Result;

use super::config::Config;
use super::model::Bart;
use crate::models::{Context, GenOpts, Generate};
use crate::pipelines::generate;

/// BART behind the `Generate` capability.
///
/// `Bart` itself cannot implement the trait: generation mutates its KV cache, so it needs
/// `&mut self` while the trait -- being cached as `Arc<dyn Generate>` and shared across threads --
/// hands out `&self`. The mutex is what reconciles the two, and it lives here rather than in the
/// model so the architecture stays a plain candle module.
pub struct Summarizer {
    model: Mutex<Bart>,
    generation: generate::Config,
    max_position_embeddings: usize,
}

impl Summarizer {
    pub fn new(config: &Config, vars: VarBuilder) -> Result<Self> {
        Ok(Self {
            model: Mutex::new(Bart::new(config, vars).map_err(error::ai)?),
            generation: generate::Config::from(config),
            max_position_embeddings: config.max_position_embeddings,
        })
    }

    pub fn beams(mut self, beams: usize) -> Self {
        self.generation = self.generation.beams(beams);
        self
    }
}

impl Generate for Summarizer {
    fn generate(&self, cx: &Context, text: &[&str], opts: &GenOpts) -> Result<Vec<String>> {
        let mut generation = self.generation;

        if let Some(beams) = opts.beams {
            generation = generation.beams(beams);
        }
        if let Some(max_len) = opts.max_len {
            generation = generation.max_length(max_len);
        }

        text.iter().map(|text| self.one(cx, text, &generation)).collect()
    }
}

impl Summarizer {
    fn one(&self, cx: &Context, text: &str, generation: &generate::Config) -> Result<String> {
        let encoding = cx.encode_one(text)?;
        let ids = encoding.get_ids();

        // The learned positional embeddings only cover `max_position_embeddings`.
        let ids = &ids[..ids.len().min(self.max_position_embeddings)];
        let input = Tensor::from_slice(ids, (1, ids.len()), cx.device()).map_err(error::ai)?;
        let mut model = self
            .model
            .lock()
            .map_err(|_| error::ai("summarization model lock poisoned".to_string()))?;

        let tokens = generate::run(&mut model, generation, &input, cx.device())?;

        cx.tokenizer()?
            .decode(&tokens, true)
            .map(|summary| summary.trim().to_string())
            .map_err(error::ai)
    }
}
