use super::char_offset;
use crate::types::Offset;

/// Locates `needle` in `text` so a remote result can carry a real span.
///
/// A hosted API returns entity *text*, never offsets — unlike a tokenizer, which gives them as
/// ground truth. So spans from a remote transport are best-effort: exact for a unique substring,
/// the first occurrence when the text repeats, and empty when the model paraphrased and the text
/// is not verbatim.
pub fn find(text: &str, needle: &str) -> Option<Offset> {
    all(text, needle).into_iter().next()
}

/// Every occurrence of `needle`, as char offsets.
pub fn all(text: &str, needle: &str) -> Vec<Offset> {
    if needle.is_empty() {
        return Vec::new();
    }

    let mut offsets = Vec::new();
    let mut cursor = 0;

    while let Some(index) = text[cursor..].find(needle) {
        let start = cursor + index;
        let end = start + needle.len();

        offsets.push(Offset::new(char_offset(text, start), char_offset(text, end)));
        cursor = end;
    }

    offsets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_unique_substring() {
        assert_eq!(find("Satya works at Microsoft", "Microsoft"), Some(Offset::new(15, 24)));
    }

    #[test]
    fn anchors_a_repeat_to_the_first_occurrence() {
        assert_eq!(find("Apple sued Apple", "Apple"), Some(Offset::new(0, 5)));
    }

    #[test]
    fn finds_every_occurrence() {
        assert_eq!(all("Apple sued Apple", "Apple"), vec![Offset::new(0, 5), Offset::new(11, 16)]);
    }

    /// A span is never invented: `(0, 0)` would be indistinguishable from a real match at the
    /// start of the document.
    #[test]
    fn yields_nothing_when_not_verbatim() {
        assert_eq!(find("Satya works at Microsoft", "Microsoft Corp"), None);
    }

    #[test]
    fn counts_chars_not_bytes() {
        assert_eq!(find("héllo wörld", "wörld"), Some(Offset::new(6, 11)));
    }
}
