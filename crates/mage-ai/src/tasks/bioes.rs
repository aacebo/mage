use super::char_offset;
use crate::models::Word;
use crate::types::{Entity, Offset};

/// BIOES decoding, distinct from the plain IOB1 aggregation used for NER: it honours `E-`/`S-`
/// tags, merges adjacent same-label entities, and weights merged scores by token count.
pub fn entities(words: Vec<Word>, text: &str, min_score: f64) -> Vec<Entity> {
    let mut entities = Entities::new(text, min_score);

    for word in words {
        entities.push(word);
    }

    entities.finish()
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Tag {
    Begin,
    Inside,
    End,
    Single,
}

impl Tag {
    fn parse(label: &str) -> Option<(Self, &str)> {
        let (tag, label) = match label.split_once('-') {
            Some(("B", label)) => (Self::Begin, label),
            Some(("I", label)) => (Self::Inside, label),
            Some(("E", label)) => (Self::End, label),
            Some(("S", label)) => (Self::Single, label),
            Some(("O", _)) | None if label == "O" => return None,
            Some(_) => return None,
            None => (Self::Single, label),
        };

        (!label.is_empty()).then_some((tag, label))
    }

    fn is_start(self) -> bool {
        matches!(self, Self::Begin | Self::Single)
    }

    fn is_terminal(self) -> bool {
        matches!(self, Self::End | Self::Single)
    }
}

struct Entities<'a> {
    text: &'a str,
    min_score: f64,
    current: Option<Builder>,
    finished: Vec<Finished>,
}

impl<'a> Entities<'a> {
    fn new(text: &'a str, min_score: f64) -> Self {
        Self {
            text,
            min_score,
            current: None,
            finished: Vec::new(),
        }
    }

    fn push(&mut self, word: Word) {
        let Some((tag, label)) = Tag::parse(&word.label) else {
            self.finish_current();
            return;
        };

        if self.should_start(tag, label, word.start) {
            self.finish_current();
            self.current = Some(Builder::new(&word, tag, label));
        } else if let Some(current) = self.current.as_mut() {
            current.push(&word, tag);
        }

        if tag.is_terminal() {
            self.finish_current();
        }
    }

    fn finish(mut self) -> Vec<Entity> {
        self.finish_current();
        self.finished.into_iter().map(|entity| entity.entity).collect()
    }

    fn should_start(&self, tag: Tag, label: &str, start: usize) -> bool {
        match self.current.as_ref() {
            Some(current) => {
                if current.is_closed() || !current.has_label(label) {
                    return true;
                }

                tag.is_start() && current.end != start
            }
            None => true,
        }
    }

    /// Filters before merging: a below-threshold word must not be merged into an adjacent
    /// entity, where it would drag that entity's averaged score down (or be resurrected by it).
    fn finish_current(&mut self) {
        let Some(builder) = self.current.take() else {
            return;
        };
        let entity = builder.finish(self.text);

        if entity.entity.score < self.min_score {
            return;
        }

        if let Some(previous) = self.finished.last_mut()
            && previous.can_merge(&entity)
        {
            previous.merge(entity, self.text);
            return;
        }

        self.finished.push(entity);
    }
}

struct Finished {
    entity: Entity,
    words: usize,
    start: usize,
    end: usize,
}

impl Finished {
    fn can_merge(&self, other: &Self) -> bool {
        self.entity.label.eq_ignore_ascii_case(&other.entity.label) && self.end == other.start
    }

    fn merge(&mut self, other: Self, text: &str) {
        let total = self.words + other.words;

        self.entity.score = ((self.entity.score * self.words as f64) + (other.entity.score * other.words as f64)) / total as f64;
        self.entity.offset.end = other.entity.offset.end;
        self.words = total;
        self.end = other.end;
        self.entity.word = text[self.start..self.end].to_string();
    }
}

struct Builder {
    label: String,
    scores: f64,
    count: usize,
    start: usize,
    end: usize,
    previous: Tag,
}

impl Builder {
    fn new(word: &Word, tag: Tag, label: &str) -> Self {
        Self {
            label: label.to_lowercase(),
            scores: word.score,
            count: 1,
            start: word.start,
            end: word.end,
            previous: tag,
        }
    }

    fn has_label(&self, label: &str) -> bool {
        self.label.eq_ignore_ascii_case(label)
    }

    fn is_closed(&self) -> bool {
        self.previous.is_terminal()
    }

    fn push(&mut self, word: &Word, tag: Tag) {
        self.scores += word.score;
        self.count += 1;
        self.end = word.end;
        self.previous = tag;
    }

    fn finish(self, text: &str) -> Finished {
        Finished {
            entity: Entity {
                word: text[self.start..self.end].to_string(),
                label: self.label,
                score: self.scores / self.count as f64,
                offset: Offset::new(char_offset(text, self.start), char_offset(text, self.end)),
            },
            words: self.count,
            start: self.start,
            end: self.end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(label: &str, score: f64, start: usize, end: usize) -> Word {
        Word {
            label: label.to_string(),
            score,
            start,
            end,
        }
    }

    /// Words of one entity (`B-` then `I-`) average into a single span, exactly as before.
    #[test]
    fn averages_words_within_an_entity() {
        let text = "John Smith";
        let words = vec![word("B-per", 0.9, 0, 4), word("I-per", 0.5, 5, 10)];

        let found = entities(words, text, 0.0);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].word, "John Smith");
        assert!((found[0].score - 0.7).abs() < 1e-9, "score was {}", found[0].score);
    }

    /// A separate below-threshold entity must be dropped BEFORE the adjacency merge, so it cannot
    /// drag a strong neighbour's averaged score down.
    #[test]
    fn filters_before_merging_adjacent_entities() {
        let text = "JohnSmith";
        let words = vec![word("S-per", 0.9, 0, 4), word("S-per", 0.1, 4, 9)];

        let found = entities(words, text, 0.5);

        assert_eq!(found.len(), 1, "the weak entity must not merge in");
        assert_eq!(found[0].word, "John");
        assert!((found[0].score - 0.9).abs() < 1e-9, "score was {}", found[0].score);
    }

    /// Contiguous spans merge by re-slicing the source, so internal punctuation survives.
    #[test]
    fn merges_contiguous_spans_from_the_source() {
        let text = "Jean-Luc";
        let words = vec![word("S-per", 0.9, 0, 4), word("S-per", 0.9, 4, 8)];

        let found = entities(words, text, 0.0);

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].word, "Jean-Luc");
    }

    #[test]
    fn drops_everything_below_the_threshold() {
        let text = "John";
        let words = vec![word("S-per", 0.1, 0, 4)];

        assert!(entities(words, text, 0.5).is_empty());
    }
}
