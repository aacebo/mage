pub mod aggregation;
pub mod anchor;
pub mod bioes;

mod candidates;
mod keybert;
mod scorer;
mod stopwords;

use mage_error::Result;

use crate::models::{Classify, Context, Embed, GenOpts, Generate, TokenClassify};
use crate::types::{Entity, Keyword, Polarity, Sentiment};

/// The user-facing routines, each a presentation over one model capability.
///
/// They are free functions, not traits: a task is something you do *with* a capable model, not a
/// capability a model has. Writing them as blanket impls would also collide -- a hosted model has
/// every capability, so `impl<T: Embed> Keywords for T` conflicts with its own keyword path.
pub fn embed<E: Embed + ?Sized>(model: &E, cx: &Context, text: &[&str]) -> Result<Vec<Vec<f32>>> {
    model.embed(cx, text)
}

/// KeyBERT, over any embedder -- local or hosted. Previously this only worked locally, by
/// special-casing a call into the embeddings pipeline's cache.
pub fn keywords<E: Embed + ?Sized>(model: &E, cx: &Context, text: &[&str], top_n: usize) -> Result<Vec<Vec<Keyword>>> {
    keybert::keywords(model, cx, text, top_n)
}

/// Sentiment is a *presentation* of sequence classification: the model returns a label, and this
/// maps it onto the binary polarity the routine promises.
pub fn sentiment<C: Classify + ?Sized>(model: &C, cx: &Context, text: &[&str]) -> Result<Vec<Sentiment>> {
    Ok(model
        .classify(cx, text)?
        .into_iter()
        .map(|label| Sentiment {
            polarity: Polarity::from_label(&label.label),
            score: label.score,
        })
        .collect())
}

pub fn entities<T: TokenClassify + ?Sized>(model: &T, cx: &Context, text: &[&str]) -> Result<Vec<Vec<Entity>>> {
    model.entities(cx, text)
}

pub fn pii<T: TokenClassify + ?Sized>(model: &T, cx: &Context, text: &[&str], min_score: f64) -> Result<Vec<Vec<Entity>>> {
    model.pii(cx, text, min_score)
}

pub fn summarize<G: Generate + ?Sized>(model: &G, cx: &Context, text: &[&str], opts: &GenOpts) -> Result<Vec<String>> {
    model.generate(cx, text, opts)
}

/// Char offsets, not byte offsets: a span is reported in the units the caller sees.
pub(crate) fn char_offset(text: &str, byte_offset: usize) -> u32 {
    let byte_offset = byte_offset.min(text.len());
    text.char_indices().take_while(|(index, _)| *index < byte_offset).count() as u32
}
