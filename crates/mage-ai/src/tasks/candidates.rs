use super::stopwords;
use crate::types::Offset;

const MIN_LENGTH: usize = 3;

pub struct Candidate {
    pub text: String,
    pub offsets: Vec<Offset>,
}

/// Distinct, non-stopword words, each carrying every offset it occurs at. Offsets are CHAR
/// offsets, matching what the remote transport re-anchors.
pub fn extract(text: &str) -> Vec<Candidate> {
    let mut candidates: Vec<Candidate> = Vec::new();

    for (start, word) in words(text) {
        let lowered = word.to_lowercase();

        if lowered.chars().count() < MIN_LENGTH || stopwords::contains(&lowered) {
            continue;
        }

        let offset = Offset::new(char_offset(text, start), char_offset(text, start + word.len()));

        match candidates.iter_mut().find(|candidate| candidate.text == lowered) {
            Some(candidate) => candidate.offsets.push(offset),
            None => candidates.push(Candidate {
                text: lowered,
                offsets: vec![offset],
            }),
        }
    }

    candidates
}

fn char_offset(text: &str, byte_offset: usize) -> u32 {
    let byte_offset = byte_offset.min(text.len());
    text.char_indices().take_while(|(index, _)| *index < byte_offset).count() as u32
}

/// Alphanumeric runs paired with their starting byte offset.
fn words(text: &str) -> Vec<(usize, &str)> {
    let mut words = Vec::new();
    let mut start: Option<usize> = None;

    for (index, ch) in text.char_indices() {
        match (ch.is_alphanumeric(), start) {
            (true, None) => start = Some(index),
            (false, Some(begin)) => {
                words.push((begin, &text[begin..index]));
                start = None;
            }
            _ => {}
        }
    }

    if let Some(begin) = start {
        words.push((begin, &text[begin..]));
    }

    words
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_are_char_not_byte() {
        // "café" is 5 bytes but 4 chars, so a byte offset would shift everything after it.
        let text = "café supports parsing";
        let found = extract(text);
        let supports = found.iter().find(|c| c.text == "supports").expect("found");
        let offset = supports.offsets[0];

        let chars: Vec<char> = text.chars().collect();
        let slice: String = chars[offset.begin as usize..offset.end as usize].iter().collect();

        assert_eq!(slice, "supports", "span sliced by CHAR must land on the word");
    }
}
