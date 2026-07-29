use super::config::Config;

/// Logits processors, applied in log space to one beam's row.
pub fn apply(row: &mut [f32], config: &Config, sequence: &[u32], generated: usize) {
    if generated == 0
        && let Some(bos) = config.forced_bos_token_id
    {
        force(row, bos as usize);
        return;
    }

    if generated + 1 >= config.max_length
        && let Some(eos) = config.forced_eos_token_id
    {
        force(row, eos as usize);
        return;
    }

    let eos = config.eos_token_id as usize;

    if generated < config.min_length && eos < row.len() {
        row[eos] = f32::NEG_INFINITY;
    }

    ban_repeat_ngrams(row, config.no_repeat_ngram_size, sequence);
}

fn force(row: &mut [f32], token: usize) {
    for (index, value) in row.iter_mut().enumerate() {
        *value = if index == token { 0.0 } else { f32::NEG_INFINITY };
    }
}

/// Bans any token that would complete an n-gram already present in this beam.
fn ban_repeat_ngrams(row: &mut [f32], size: usize, sequence: &[u32]) {
    if size == 0 || sequence.len() < size {
        return;
    }

    let prefix = &sequence[sequence.len() + 1 - size..];

    for window in sequence.windows(size) {
        let (seen, next) = window.split_at(size - 1);

        if seen == prefix
            && let Some(value) = row.get_mut(next[0] as usize)
        {
            *value = f32::NEG_INFINITY;
        }
    }
}
