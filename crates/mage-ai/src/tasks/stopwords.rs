const STOPWORDS: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "all", "any", "can", "had", "her", "was", "one", "our", "out", "day", "get",
    "has", "him", "his", "how", "man", "new", "now", "old", "see", "two", "way", "who", "boy", "did", "its", "let", "put", "say",
    "she", "too", "use", "that", "with", "have", "this", "will", "your", "from", "they", "know", "want", "been", "good", "much",
    "some", "time", "very", "when", "come", "here", "just", "like", "long", "make", "many", "over", "such", "take", "than",
    "them", "well", "were", "what", "would", "there", "their", "about", "which", "these", "could", "other", "into", "only",
    "also", "back", "after", "first", "because",
];

pub fn contains(word: &str) -> bool {
    STOPWORDS.contains(&word)
}
