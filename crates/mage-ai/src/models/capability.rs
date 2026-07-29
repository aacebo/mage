use candle_core::Device;
use mage_error::Result;
use tokenizers::Tokenizer;

use crate::pipelines::common::Batch;
use crate::types::Entity;

/// Borrows what a model needs to turn text into tensors. A loaded model owns its weights; the
/// tokenizer and device are lent per call, so one set of weights can serve several capabilities.
///
/// The tokenizer is optional because a hosted model has none -- it is handed text and returns
/// text. Its capability impls never touch the context, and `tokenizer()` is the error a local
/// model would raise if its tokenizer were somehow missing.
pub struct Context<'a> {
    name: &'a str,
    device: &'a Device,
    tokenizer: Option<&'a Tokenizer>,
}

impl<'a> Context<'a> {
    pub fn new(tokenizer: Option<&'a Tokenizer>, device: &'a Device, name: &'a str) -> Self {
        Self { tokenizer, device, name }
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn device(&self) -> &Device {
        self.device
    }

    pub fn tokenizer(&self) -> Result<&Tokenizer> {
        self.tokenizer
            .ok_or_else(|| mage_error::ai(format!("{} has no tokenizer", self.name)))
    }

    pub fn encode(&self, text: &[&str]) -> Result<Batch> {
        let encodings = self.tokenizer()?.encode_batch(text.to_vec(), true).map_err(mage_error::ai)?;
        Batch::new(encodings, self.device)
    }

    pub fn encode_one(&self, text: &str) -> Result<tokenizers::Encoding> {
        self.tokenizer()?.encode(text, true).map_err(mage_error::ai)
    }
}

/// One label for a whole input.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    pub label: String,
    pub score: f64,
}

/// A whole word: sub-word pieces already merged, carrying BYTE offsets into the source text.
#[derive(Debug, Clone)]
pub struct Word {
    pub label: String,
    pub score: f64,
    pub start: usize,
    pub end: usize,
}

pub struct GenOpts {
    pub prompt: &'static str,
    pub max_len: Option<usize>,
    pub beams: Option<usize>,
}

impl Default for GenOpts {
    fn default() -> Self {
        Self {
            prompt: "Summarize the text the user gives you. Be concise and factual; use only \
                     information present in the text.",
            max_len: None,
            beams: None,
        }
    }
}

/// A sentence vector per input.
pub trait Embed: Send + Sync {
    fn embed(&self, cx: &Context, text: &[&str]) -> Result<Vec<Vec<f32>>>;
}

/// Sequence classification: one label per input.
pub trait Classify: Send + Sync {
    fn classify(&self, cx: &Context, text: &[&str]) -> Result<Vec<Label>>;
}

/// Token classification: labelled spans per input.
///
/// Two methods, because the two tasks want different decodes of the same forward pass. A local
/// model labels sub-word tokens, and NER (IOB1) and PII (BIOES) disagree on how to stitch those
/// into spans -- so both decodes live here rather than one being derived from the other. A hosted
/// model has no tokens at all; it returns spans directly and decodes nothing.
pub trait TokenClassify: Send + Sync {
    fn entities(&self, cx: &Context, text: &[&str]) -> Result<Vec<Vec<Entity>>>;

    fn pii(&self, cx: &Context, text: &[&str], min_score: f64) -> Result<Vec<Vec<Entity>>>;
}

/// Text in, text out. BART does this by beam search; a hosted model by its completion endpoint.
pub trait Generate: Send + Sync {
    fn generate(&self, cx: &Context, text: &[&str], opts: &GenOpts) -> Result<Vec<String>>;
}
