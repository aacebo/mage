mod cache;

pub mod common;
pub mod generate;

pub use cache::{Cache, Key};
pub use common::Batch;
use mage_error::Result;

use crate::models::{GenOpts, Model, ModelId, ModelRef, Provider};
use crate::tasks;
use crate::types::{Annotation, Artifact, ArtifactContent, Entity, Offset};

static MODELS: std::sync::LazyLock<Cache<crate::models::Model>> = std::sync::LazyLock::new(Cache::new);
const TOP_N: usize = 5;

type TokenArgs = (Vec<String>, f64, std::sync::Arc<Model>);

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextArgs {
    pub text: Vec<String>,
    pub model: ModelArgs,
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct SummarizeArgs {
    pub text: Vec<String>,
    pub model: ModelArgs,
    #[serde(default)]
    pub beams: Option<usize>,
    #[serde(default)]
    pub max_len: Option<usize>,
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScoredArgs {
    pub text: Vec<String>,
    pub min_score: f64,
    pub model: ModelArgs,
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelArgs {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

impl ModelArgs {
    pub fn resolve(self, default: ModelRef) -> Result<ModelRef> {
        let Some(provider) = self.provider else {
            let Some(model) = self.model else {
                return Ok(default);
            };

            return model.parse();
        };

        let provider: Provider = provider.parse()?;
        let id: ModelId = self
            .model
            .ok_or(mage_error::ai("model is required when provider is set"))?
            .parse()?;

        Ok(ModelRef::remote(provider, id).base_url(self.base_url))
    }
}

pub fn borrow(text: &[String]) -> Vec<&str> {
    text.iter().map(String::as_str).collect()
}

/// One cache of loaded models, keyed by `(model, api_key)` -- not one per capability. A model used
/// for two routines now loads its weights once.
pub fn load(model: &ModelRef, api_key: &Option<String>) -> Result<std::sync::Arc<crate::models::Model>> {
    use candle_core::{DType, Device};

    MODELS.get_or_build(Key::new(model, api_key), || {
        Ok(std::sync::Arc::new(crate::models::Model::new(
            model,
            api_key,
            Device::Cpu,
            DType::F32,
        )?))
    })
}

// Each routine is the same four steps: parse args, load the model, ask it for the capability the
// routine needs, and present the result. Asking is where the capability matrix bites -- a model
// that cannot do the task fails here, by name, instead of part-way through inference.

pub fn embeddings(args: TextArgs) -> Result<Vec<Artifact>> {
    let api_key = args.model.api_key.clone();
    let model = load(&args.model.resolve(defaults::embed())?, &api_key)?;
    let capable = model.as_embed().ok_or_else(|| model.cannot("embed"))?;
    let out = tasks::embed(capable, &model.context(), &borrow(&args.text))?;
    let artifacts: Vec<Artifact> = out
        .into_iter()
        .zip(args.text)
        .map(|(vector, text)| Artifact {
            name: "embedding".to_string(),
            value: ArtifactContent::text(text),
            vector: Some(vector),
        })
        .collect();

    Ok(artifacts)
}

pub fn keywords(args: ScoredArgs) -> Result<Vec<Annotation>> {
    let api_key = args.model.api_key.clone();
    let model = load(&args.model.resolve(defaults::keywords())?, &api_key)?;
    let capable = model.as_embed().ok_or_else(|| model.cannot("embed"))?;
    let out = tasks::keywords(capable, &model.context(), &borrow(&args.text), TOP_N)?;
    let min_score = args.min_score as f32;
    let mut annotations: Vec<Annotation> = Vec::new();

    for keywords in out {
        for keyword in keywords.into_iter().filter(|k| k.score >= min_score) {
            annotations.push(Annotation {
                name: String::from("keyword"),
                label: keyword.text.clone(),
                text: keyword.text,
                score: keyword.score as f64,
                spans: keyword.offsets.iter().map(|o| Offset::new(o.begin, o.end)).collect(),
            });
        }
    }

    Ok(annotations)
}

pub fn sentiment(args: ScoredArgs) -> Result<Vec<Annotation>> {
    let api_key = args.model.api_key.clone();
    let model = load(&args.model.resolve(defaults::classify())?, &api_key)?;
    let capable = model.as_classify().ok_or_else(|| model.cannot("classify"))?;
    let out = tasks::sentiment(capable, &model.context(), &borrow(&args.text))?;
    let mut annotations: Vec<Annotation> = Vec::new();

    for (index, sentiment) in out.into_iter().enumerate() {
        if sentiment.score < args.min_score {
            continue;
        }

        let source = args.text.get(index).map(String::as_str).unwrap_or_default();
        let label = sentiment.polarity.as_str();

        annotations.push(Annotation {
            name: String::from("sentiment"),
            label: label.to_string(),
            text: label.to_string(),
            score: sentiment.score,
            spans: vec![Offset::new(0, source.chars().count() as u32)],
        });
    }

    Ok(annotations)
}

pub fn entities(args: ScoredArgs) -> Result<Vec<Annotation>> {
    let (text, min_score, model) = token_args(args)?;
    let capable = model.as_token_classify().ok_or_else(|| model.cannot("token-classify"))?;
    let out = tasks::entities(capable, &model.context(), &borrow(&text))?;
    let mut annotations: Vec<Annotation> = Vec::new();

    for entities in out {
        for entity in entities.into_iter().filter(|e| e.score >= min_score) {
            annotations.push(Annotation {
                name: "entity".to_string(),
                text: entity.word,
                score: entity.score,
                spans: vec![Offset::new(entity.offset.begin, entity.offset.end)],
                label: match entity.label.as_str() {
                    "ORG" => "organization".to_string(),
                    "PER" => "person".to_string(),
                    "LOC" => "location".to_string(),
                    other => other.to_lowercase(),
                },
            });
        }
    }

    Ok(annotations)
}

pub fn pii(args: ScoredArgs) -> Result<Vec<Vec<Entity>>> {
    let (text, min_score, model) = token_args(args)?;
    let capable = model.as_token_classify().ok_or_else(|| model.cannot("token-classify"))?;
    let out = tasks::pii(capable, &model.context(), &borrow(&text), min_score)?;
    Ok(out)
}

pub fn summarize(args: SummarizeArgs) -> Result<Vec<Artifact>> {
    let api_key = args.model.api_key.clone();
    let model = load(&args.model.resolve(defaults::generate())?, &api_key)?;
    let capable = model.as_generate().ok_or_else(|| model.cannot("generate"))?;
    let opts = GenOpts {
        beams: args.beams,
        max_len: args.max_len,
        ..GenOpts::default()
    };
    let out = tasks::summarize(capable, &model.context(), &borrow(&args.text), &opts)?;
    let artifacts: Vec<Artifact> = out
        .into_iter()
        .map(|summary| Artifact {
            name: "summary".to_string(),
            value: ArtifactContent::text(summary),
            vector: None,
        })
        .collect();

    Ok(artifacts)
}

fn token_args(args: ScoredArgs) -> Result<TokenArgs> {
    let api_key = args.model.api_key.clone();
    let model = load(&args.model.resolve(defaults::token_classify())?, &api_key)?;
    Ok((args.text, args.min_score, model))
}

/// Default models, one per capability. A model that cannot serve the routine you called is an
/// error -- the empty cells of the capability matrix.
pub mod defaults {
    use crate::models::{ModelId, ModelRef};

    pub fn hub(repo: &str) -> ModelRef {
        ModelRef::hub(repo.parse::<ModelId>().expect("built-in model ids are valid"))
    }

    pub fn embed() -> ModelRef {
        hub("sentence-transformers/all-MiniLM-L12-v2")
    }

    pub fn keywords() -> ModelRef {
        hub("sentence-transformers/all-MiniLM-L6-v2")
    }

    pub fn classify() -> ModelRef {
        hub("distilbert-base-uncased-finetuned-sst-2-english")
    }

    pub fn token_classify() -> ModelRef {
        hub("dbmdz/bert-large-cased-finetuned-conll03-english")
    }

    pub fn generate() -> ModelRef {
        hub("facebook/bart-large-cnn")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(model: &str) -> ModelArgs {
        ModelArgs {
            provider: None,
            model: Some(model.to_string()),
            base_url: None,
            api_key: None,
        }
    }

    /// The defaults differ per capability, so a routine left to itself embeds with one model and
    /// extracts keywords with another. That is what makes the next test worth writing.
    #[test]
    fn the_capability_defaults_are_different_models() {
        assert_ne!(defaults::embed(), defaults::keywords());
    }

    /// An explicit `model=` overrides the per-capability default, so `ai.embeddings` and
    /// `ai.keywords` on one model resolve to one cache key -- and the weights load once. Under a
    /// cache keyed by capability rather than by model, these keys differed and the weights loaded
    /// twice.
    #[test]
    fn two_routines_on_one_model_share_a_cache_key() {
        let name = "sentence-transformers/all-MiniLM-L12-v2";

        // `embeddings` and `keywords` resolve the same args against their own default.
        let embed = args(name).resolve(defaults::embed()).unwrap();
        let keywords = args(name).resolve(defaults::keywords()).unwrap();

        assert_eq!(embed, keywords);
        assert_eq!(Key::new(&embed, &None), Key::new(&keywords, &None));
    }

    /// Without `model=`, each routine falls back to its own default -- so they do NOT share.
    #[test]
    fn routines_left_on_their_defaults_do_not_share_a_cache_key() {
        let embed = ModelArgs {
            provider: None,
            model: None,
            base_url: None,
            api_key: None,
        }
        .resolve(defaults::embed())
        .unwrap();

        let keywords = ModelArgs {
            provider: None,
            model: None,
            base_url: None,
            api_key: None,
        }
        .resolve(defaults::keywords())
        .unwrap();

        assert_ne!(Key::new(&embed, &None), Key::new(&keywords, &None));
    }
}
