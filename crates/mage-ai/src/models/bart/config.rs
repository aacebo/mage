#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub vocab_size: usize,
    pub d_model: usize,
    pub encoder_layers: usize,
    pub decoder_layers: usize,
    pub encoder_attention_heads: usize,
    pub decoder_attention_heads: usize,
    pub encoder_ffn_dim: usize,
    pub decoder_ffn_dim: usize,
    pub activation_function: candle_nn::Activation,
    pub max_position_embeddings: usize,
    pub eos_token_id: u32,
    pub decoder_start_token_id: u32,
    pub forced_bos_token_id: Option<u32>,
    pub forced_eos_token_id: Option<u32>,
    #[serde(default)]
    pub scale_embedding: bool,
    #[serde(default = "default_beams")]
    pub num_beams: usize,
    #[serde(default = "default_max_length")]
    pub max_length: usize,
    #[serde(default)]
    pub min_length: usize,
    #[serde(default = "default_length_penalty")]
    pub length_penalty: f64,
    #[serde(default)]
    pub no_repeat_ngram_size: usize,
}

impl Config {
    pub fn vocab_size(mut self, vocab_size: usize) -> Self {
        self.vocab_size = vocab_size;
        self
    }

    pub fn d_model(mut self, d_model: usize) -> Self {
        self.d_model = d_model;
        self
    }

    pub fn encoder_layers(mut self, layers: usize) -> Self {
        self.encoder_layers = layers;
        self
    }

    pub fn decoder_layers(mut self, layers: usize) -> Self {
        self.decoder_layers = layers;
        self
    }

    pub fn max_position_embeddings(mut self, positions: usize) -> Self {
        self.max_position_embeddings = positions;
        self
    }

    pub fn scale_embedding(mut self, scale: bool) -> Self {
        self.scale_embedding = scale;
        self
    }

    pub fn num_beams(mut self, beams: usize) -> Self {
        self.num_beams = beams;
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

    pub fn length_penalty(mut self, penalty: f64) -> Self {
        self.length_penalty = penalty;
        self
    }

    pub fn no_repeat_ngram_size(mut self, size: usize) -> Self {
        self.no_repeat_ngram_size = size;
        self
    }
}

fn default_beams() -> usize {
    4
}

fn default_max_length() -> usize {
    142
}

fn default_length_penalty() -> f64 {
    2.0
}
