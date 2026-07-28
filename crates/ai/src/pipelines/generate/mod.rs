mod config;
mod constraints;

use candle_core::{Device, Tensor};
use candle_nn::ops;
pub use config::Config;
use error::Result;

use crate::models::bart::Bart;

pub fn run(model: &mut Bart, config: &Config, input: &Tensor, device: &Device) -> Result<Vec<u32>> {
    let beams = config.beams;

    model.reset_kv_cache();

    let encoder_xs = model.encode(input).map_err(error::ai)?;
    let (_, seq, hidden) = encoder_xs.dims3().map_err(error::ai)?;
    let encoder_xs = encoder_xs
        .expand((beams, seq, hidden))
        .and_then(|xs| xs.contiguous())
        .map_err(error::ai)?;

    let mut sequences: Vec<Vec<u32>> = vec![vec![config.decoder_start_token_id]; beams];
    let mut scores: Vec<f64> = (0..beams).map(|i| if i == 0 { 0.0 } else { f64::NEG_INFINITY }).collect();
    let mut hypotheses: Vec<(f64, Vec<u32>)> = Vec::new();
    let mut tokens = vec![config.decoder_start_token_id; beams];

    for past_kv_len in 0..config.max_length {
        let input = Tensor::from_vec(tokens.clone(), (beams, 1), device).map_err(error::ai)?;

        let logits = model
            .decode(&input, &encoder_xs, past_kv_len)
            .and_then(|logits| logits.squeeze(1))
            .map_err(error::ai)?;
        let mut log_probs = ops::log_softmax(&logits, 1)
            .and_then(|probs| probs.to_vec2::<f32>())
            .map_err(error::ai)?;
        let generated = sequences[0].len() - 1;

        for (beam, row) in log_probs.iter_mut().enumerate() {
            constraints::apply(row, config, &sequences[beam], generated);
        }

        let mut candidates: Vec<(f64, usize, u32)> = Vec::new();

        for (beam, row) in log_probs.iter().enumerate() {
            if scores[beam].is_infinite() && scores[beam] < 0.0 {
                continue;
            }

            for (token, log_prob) in row.iter().enumerate() {
                candidates.push((scores[beam] + *log_prob as f64, beam, token as u32));
            }
        }

        // Only the top 2*beams matter (2x so beams finishing on EOS still leave `beams` live
        // continuations). Partition rather than sort all beams*vocab candidates.
        let wanted = (beams * 2).min(candidates.len());
        candidates.select_nth_unstable_by(wanted - 1, |a, b| b.0.total_cmp(&a.0));
        candidates.truncate(wanted);
        candidates.sort_by(|a, b| b.0.total_cmp(&a.0));

        let mut next: Vec<(f64, usize, u32)> = Vec::with_capacity(beams);

        for (score, beam, token) in candidates {
            if token == config.eos_token_id {
                let mut sequence = sequences[beam].clone();
                sequence.push(token);
                hypotheses.push((normalize(score, &sequence, config), sequence));
                continue;
            }

            next.push((score, beam, token));

            if next.len() == beams {
                break;
            }
        }

        if next.is_empty() || hypotheses.len() >= beams {
            break;
        }

        // Follow the surviving beams. Skipping this makes a beam read whichever history
        // happens to sit in its slot.
        let order: Vec<u32> = next.iter().map(|(_, beam, _)| *beam as u32).collect();
        let order = Tensor::from_vec(order, next.len(), device).map_err(error::ai)?;
        model.reorder_kv_cache(&order).map_err(error::ai)?;

        let previous = sequences.clone();

        sequences.clear();
        scores.clear();
        tokens.clear();

        for (score, beam, token) in &next {
            let mut sequence = previous[*beam].clone();
            sequence.push(*token);

            sequences.push(sequence);
            scores.push(*score);
            tokens.push(*token);
        }
    }

    if hypotheses.is_empty() {
        let best = scores
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(index, _)| index)
            .unwrap_or(0);

        return Ok(sequences.swap_remove(best));
    }

    hypotheses.sort_by(|a, b| b.0.total_cmp(&a.0));
    Ok(hypotheses.swap_remove(0).1)
}

/// HuggingFace's length penalty: `score / len^penalty`. Applied when a hypothesis completes,
/// never to the running beam scores.
fn normalize(score: f64, sequence: &[u32], config: &Config) -> f64 {
    score / (sequence.len() as f64).powf(config.length_penalty)
}
