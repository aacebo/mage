use mage_error::Result;
use tokenizers::Encoding;

use super::char_offset;
use crate::models::Word;
use crate::types::{Entity, Offset};

/// Merges sub-word pieces back into whole words, averaging their scores. The label of a word's
/// first piece wins, matching HuggingFace's aggregation.
pub fn words(probs: &[Vec<f32>], encoding: &Encoding, labels: &[String]) -> Result<Vec<Word>> {
    let offsets = encoding.get_offsets();
    let word_ids = encoding.get_word_ids();

    let mut words: Vec<Word> = Vec::new();
    let mut counts: Vec<usize> = Vec::new();
    let mut previous: Option<u32> = None;

    for (index, row) in probs.iter().enumerate() {
        // Special tokens ([CLS]/[SEP]) carry no word id.
        let Some(word_id) = word_ids.get(index).copied().flatten() else {
            continue;
        };

        let (label, score) = best(row, labels)?;
        let (start, end) = offsets[index];

        if previous == Some(word_id)
            && let (Some(word), Some(count)) = (words.last_mut(), counts.last_mut())
        {
            word.score += score as f64;
            word.end = word.end.max(end);
            *count += 1;
        } else {
            words.push(Word {
                label,
                score: score as f64,
                start,
                end,
            });
            counts.push(1);
        }

        previous = Some(word_id);
    }

    for (word, count) in words.iter_mut().zip(&counts) {
        word.score /= *count as f64;
    }

    Ok(words)
}

/// CoNLL-03 is IOB1: entities may start with `I-`, and `B-` only splits adjacent same-type
/// entities. Decoding it as IOB2 drops most entities.
pub fn entities(words: Vec<Word>, text: &str) -> Vec<Entity> {
    let mut entities: Vec<Entity> = Vec::new();
    let mut open: Option<Open> = None;

    for word in words {
        let Some((prefix, label)) = word.label.split_once('-') else {
            flush(open.take(), text, &mut entities);
            continue;
        };

        let continues = open.as_ref().is_some_and(|open| open.label == label);

        if prefix == "B" || !continues {
            flush(open.take(), text, &mut entities);
            open = Some(Open {
                label: label.to_string(),
                scores: vec![word.score],
                start: word.start,
                end: word.end,
            });
        } else if let Some(open) = open.as_mut() {
            open.scores.push(word.score);
            open.end = word.end;
        }
    }

    flush(open, text, &mut entities);
    entities
}

struct Open {
    label: String,
    scores: Vec<f64>,
    start: usize,
    end: usize,
}

fn flush(open: Option<Open>, text: &str, entities: &mut Vec<Entity>) {
    let Some(open) = open else { return };
    let score = open.scores.iter().sum::<f64>() / open.scores.len() as f64;

    entities.push(Entity {
        word: text[open.start..open.end].to_string(),
        score,
        label: open.label,
        offset: Offset::new(char_offset(text, open.start), char_offset(text, open.end)),
    });
}

fn best(row: &[f32], labels: &[String]) -> Result<(String, f32)> {
    let (index, score) = row
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .ok_or_else(|| mage_error::ai("empty logits row".to_string()))?;

    let label = labels
        .get(index)
        .ok_or_else(|| mage_error::ai(format!("no label for index {index}")))?;

    Ok((label.clone(), *score))
}
