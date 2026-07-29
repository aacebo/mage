use crate::models::bart;

/// Decoding parameters. Defaults come from the checkpoint's own `config.json`.
#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub beams: usize,
    pub max_length: usize,
    pub min_length: usize,
    pub length_penalty: f64,
    pub no_repeat_ngram_size: usize,
    pub eos_token_id: u32,
    pub decoder_start_token_id: u32,
    pub forced_bos_token_id: Option<u32>,
    pub forced_eos_token_id: Option<u32>,
}

impl From<&bart::Config> for Config {
    fn from(config: &bart::Config) -> Self {
        Self {
            beams: config.num_beams.max(1),
            max_length: config.max_length,
            min_length: config.min_length,
            length_penalty: config.length_penalty,
            no_repeat_ngram_size: config.no_repeat_ngram_size,
            eos_token_id: config.eos_token_id,
            decoder_start_token_id: config.decoder_start_token_id,
            forced_bos_token_id: config.forced_bos_token_id,
            forced_eos_token_id: config.forced_eos_token_id,
        }
    }
}

impl Config {
    pub fn beams(mut self, beams: usize) -> Self {
        self.beams = beams.max(1);
        self
    }

    pub fn max_length(mut self, max_length: usize) -> Self {
        self.max_length = max_length;
        self
    }

    pub fn min_length(mut self, min_length: usize) -> Self {
        self.min_length = min_length;
        self
    }

    pub fn length_penalty(mut self, length_penalty: f64) -> Self {
        self.length_penalty = length_penalty;
        self
    }
}
