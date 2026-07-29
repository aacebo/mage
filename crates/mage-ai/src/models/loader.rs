use std::path::Path;
use std::sync::Arc;

use candle_core::{DType, Device};
use candle_nn::VarBuilder;
use mage_error::Result;
use tokenizers::Tokenizer;

use super::{Repository, tokenizer};

pub struct Loader {
    repo: Arc<dyn Repository>,
    device: Device,
    dtype: DType,
}

impl Loader {
    pub fn new(repo: Arc<dyn Repository>, device: Device, dtype: DType) -> Self {
        Self { repo, device, dtype }
    }

    pub fn device(&self) -> &Device {
        &self.device
    }

    pub fn config<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        self.json("config.json")
    }

    pub fn tokenizer(&self) -> Result<Tokenizer> {
        if let Ok(path) = self.repo.resolve(Path::new("tokenizer.json")) {
            return tokenizer::from_file(path);
        }

        let path = self.repo.resolve(Path::new("vocab.txt"))?;
        tokenizer::from_vocab(path, self.lowercase()?)
    }

    pub fn vars(&self) -> Result<VarBuilder<'static>> {
        let path = self.repo.resolve(Path::new("model.safetensors"))?;

        unsafe { VarBuilder::from_mmaped_safetensors(&[path], self.dtype, &self.device) }.map_err(mage_error::ai)
    }

    pub fn json<T: serde::de::DeserializeOwned>(&self, file: &str) -> Result<T> {
        let path = self.repo.resolve(Path::new(file))?;
        let bytes = std::fs::read(path).map_err(mage_error::ai)?;

        serde_json::from_slice(&bytes).map_err(mage_error::ai)
    }

    /// ModelRef-specific: SST-2 is uncased, CoNLL-03 is cased. Lowercasing a cased model wrecks it.
    fn lowercase(&self) -> Result<bool> {
        #[derive(serde::Deserialize)]
        struct Config {
            #[serde(default)]
            do_lower_case: bool,
        }

        if !self.repo.exists(Path::new("tokenizer_config.json")) {
            return Ok(false);
        }

        let config: Config = self.json("tokenizer_config.json")?;
        Ok(config.do_lower_case)
    }
}
