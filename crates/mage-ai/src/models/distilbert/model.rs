use candle_core::Tensor;
use candle_nn::VarBuilder;
use candle_transformers::models::distilbert;
use mage_error::Result;

use super::config::Config;
use crate::models::Forward;

pub struct DistilBert {
    inner: distilbert::DistilBertModel,
}

impl DistilBert {
    pub fn new(vars: VarBuilder, config: &Config) -> Result<Self> {
        Ok(Self {
            inner: distilbert::DistilBertModel::load(vars, &config.to_candle()?).map_err(mage_error::ai)?,
        })
    }

    pub fn forward(&self, ids: &Tensor, padding: &Tensor) -> Result<Tensor> {
        self.inner.forward(ids, padding).map_err(mage_error::ai)
    }
}

impl Forward for DistilBert {
    type Input = (Tensor, Tensor);
    type Output = Tensor;

    fn forward(&self, (ids, padding): Self::Input) -> Result<Self::Output> {
        self.forward(&ids, &padding)
    }
}
