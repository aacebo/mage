use mage_error::Result;

use super::RemoteModel;
use super::capability::{Classify, Context, Embed, GenOpts, Generate, Label, TokenClassify};
use crate::tasks::anchor;
use crate::types::Entity;

const CLASSIFY: &str = "Classify the text the user gives you. Respond with JSON matching the \
                        schema: `label` is your chosen class and `score` is your confidence from \
                        0.0 to 1.0. Prefer the labels \"positive\" and \"negative\" unless the text \
                        clearly calls for another.";

const TOKENS: &str = "Extract the named entities and personally identifying information from the \
                      text the user gives you. Respond with JSON matching the schema. `label` names \
                      the kind (person, organization, location, misc, email, phone, address, ssn, \
                      credit_card, ...). `text` must be copied verbatim from the input. `score` is \
                      your confidence from 0.0 to 1.0.";

#[derive(serde::Deserialize)]
struct Classified {
    label: String,
    score: f64,
}

#[derive(serde::Deserialize)]
struct Found {
    entities: Vec<Item>,
}

#[derive(serde::Deserialize)]
struct Item {
    label: String,
    text: String,
    score: f64,
}

/// A hosted model serves every capability: the annotation ones through a JSON-schema-constrained
/// chat completion, generation through a plain completion, and embeddings through their own
/// endpoint. This is the full remote row of the capability matrix.
impl Embed for RemoteModel {
    fn embed(&self, _cx: &Context, text: &[&str]) -> Result<Vec<Vec<f32>>> {
        if text.is_empty() {
            return Ok(Vec::new());
        }

        self.client().embeddings(text)
    }
}

impl Classify for RemoteModel {
    fn classify(&self, _cx: &Context, text: &[&str]) -> Result<Vec<Label>> {
        text.iter()
            .map(|text| {
                let out: Classified = self.client().json(CLASSIFY, text, classify_schema())?;

                Ok(Label {
                    label: out.label.to_lowercase(),
                    score: out.score,
                })
            })
            .collect()
    }
}

/// A hosted model has no sub-word tokens to decode: it returns spans directly. So both methods
/// come off one prompt, and `pii` differs only by the score filter.
impl TokenClassify for RemoteModel {
    fn entities(&self, _cx: &Context, text: &[&str]) -> Result<Vec<Vec<Entity>>> {
        text.iter().map(|text| self.spans(text)).collect()
    }

    fn pii(&self, cx: &Context, text: &[&str], min_score: f64) -> Result<Vec<Vec<Entity>>> {
        let mut out = self.entities(cx, text)?;

        for entities in &mut out {
            entities.retain(|entity| entity.score >= min_score);
        }

        Ok(out)
    }
}

impl RemoteModel {
    fn spans(&self, text: &str) -> Result<Vec<Entity>> {
        let found: Found = self.client().json(TOKENS, text, tokens_schema())?;

        Ok(found
            .entities
            .into_iter()
            // A hosted API gives no offsets, so spans are re-anchored in the source. An entity
            // that is not verbatim cannot be anchored -- and violates the prompt -- so it is
            // dropped rather than given an invented span.
            .filter_map(|item| {
                Some(Entity {
                    offset: anchor::find(text, &item.text)?,
                    word: item.text,
                    label: item.label.to_lowercase(),
                    score: item.score,
                })
            })
            .collect())
    }
}

/// Generation is a plain completion, not a schema-constrained one: the output is prose, and
/// wrapping it in a JSON envelope buys nothing.
impl Generate for RemoteModel {
    fn generate(&self, _cx: &Context, text: &[&str], opts: &GenOpts) -> Result<Vec<String>> {
        text.iter()
            .map(|text| self.client().complete(opts.prompt, text, opts.max_len))
            .collect()
    }
}

fn classify_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "label": { "type": "string" },
            "score": { "type": "number" },
        },
        "required": ["label", "score"],
        "additionalProperties": false,
    })
}

fn tokens_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "entities": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "label": { "type": "string" },
                        "text": { "type": "string" },
                        "score": { "type": "number" },
                    },
                    "required": ["label", "text", "score"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["entities"],
        "additionalProperties": false,
    })
}
