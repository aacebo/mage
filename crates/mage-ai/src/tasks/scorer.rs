use super::candidates::Candidate;
use crate::types::Keyword;

/// Ranks candidates by cosine similarity to the document. Vectors are L2-normalized, so the
/// dot product is the cosine.
pub fn rank(candidates: Vec<Candidate>, document: &[f32], vectors: &[Vec<f32>], top_n: usize) -> Vec<Keyword> {
    let mut keywords: Vec<Keyword> = candidates
        .into_iter()
        .zip(vectors)
        .map(|(candidate, vector)| Keyword {
            score: dot(document, vector),
            text: candidate.text,
            offsets: candidate.offsets,
        })
        .collect();

    keywords.sort_by(|a, b| b.score.total_cmp(&a.score));
    keywords.truncate(top_n);
    keywords
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
