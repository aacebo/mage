use std::path::Path;

use mage_error::Result;
use tokenizers::models::wordpiece::WordPiece;
use tokenizers::normalizers::BertNormalizer;
use tokenizers::pre_tokenizers::bert::BertPreTokenizer;
use tokenizers::processors::bert::BertProcessing;
use tokenizers::{Model, Tokenizer};

pub fn from_file(path: impl AsRef<Path>) -> Result<Tokenizer> {
    Tokenizer::from_file(path.as_ref()).map_err(mage_error::ai)
}

/// The SST-2 and CoNLL-03 checkpoints ship a WordPiece `vocab.txt` and no `tokenizer.json`.
pub fn from_vocab(path: impl AsRef<Path>, lowercase: bool) -> Result<Tokenizer> {
    let wordpiece = WordPiece::from_file(&path.as_ref().to_string_lossy())
        .build()
        .map_err(mage_error::ai)?;
    let cls = wordpiece
        .token_to_id("[CLS]")
        .ok_or_else(|| mage_error::ai("vocab is missing [CLS]".to_string()))?;
    let sep = wordpiece
        .token_to_id("[SEP]")
        .ok_or_else(|| mage_error::ai("vocab is missing [SEP]".to_string()))?;

    let mut tokenizer = Tokenizer::new(wordpiece);

    tokenizer
        .with_normalizer(Some(BertNormalizer::new(true, true, None, lowercase)))
        .with_pre_tokenizer(Some(BertPreTokenizer))
        .with_post_processor(Some(BertProcessing::new(
            ("[SEP]".to_string(), sep),
            ("[CLS]".to_string(), cls),
        )));

    Ok(tokenizer)
}
