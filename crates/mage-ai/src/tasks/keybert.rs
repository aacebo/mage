use mage_error::Result;

use super::{candidates, scorer};
use crate::models::{Context, Embed};
use crate::types::Keyword;

/// KeyBERT: embed the document, embed each candidate phrase, rank candidates by cosine similarity
/// to the document. Any embedder will do -- which is the point of taking `&dyn Embed` rather than
/// a concrete model.
pub fn keywords<E: Embed + ?Sized>(model: &E, cx: &Context, text: &[&str], top_n: usize) -> Result<Vec<Vec<Keyword>>> {
    text.iter().map(|text| one(model, cx, text, top_n)).collect()
}

fn one<E: Embed + ?Sized>(model: &E, cx: &Context, text: &str, top_n: usize) -> Result<Vec<Keyword>> {
    let candidates = candidates::extract(text);

    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    // The document and its candidates go through in one batch, so the document vector and the
    // candidate vectors come from the same forward pass.
    let mut batch: Vec<&str> = Vec::with_capacity(candidates.len() + 1);
    batch.push(text);
    batch.extend(candidates.iter().map(|candidate| candidate.text.as_str()));

    let vectors = model.embed(cx, &batch)?;
    let Some((document, vectors)) = vectors.split_first() else {
        return Ok(Vec::new());
    };

    Ok(scorer::rank(candidates, document, vectors, top_n))
}
