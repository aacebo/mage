use candle_core::{Device, Tensor};
use mage_error::Result;
use tokenizers::Encoding;

pub struct Batch {
    pub ids: Tensor,
    pub mask: Tensor,
}

impl Batch {
    /// Pads a set of encodings to a common length and stacks them into `(batch, seq)` tensors.
    /// `mask` is the standard keep-mask: 1 for real tokens, 0 for padding.
    pub fn new(encodings: Vec<Encoding>, device: &Device) -> Result<Self> {
        let width = encodings.iter().map(|e| e.get_ids().len()).max().unwrap_or(0);
        let mut ids: Vec<u32> = Vec::with_capacity(encodings.len() * width);
        let mut mask: Vec<u32> = Vec::with_capacity(encodings.len() * width);

        for encoding in &encodings {
            let encoded = encoding.get_ids();

            ids.extend_from_slice(encoded);
            mask.extend_from_slice(encoding.get_attention_mask());

            for _ in encoded.len()..width {
                ids.push(0);
                mask.push(0);
            }
        }

        let shape = (encodings.len(), width);

        Ok(Self {
            ids: Tensor::from_vec(ids, shape, device).map_err(mage_error::ai)?,
            mask: Tensor::from_vec(mask, shape, device).map_err(mage_error::ai)?,
        })
    }

    /// The inverse of `mask`: 1 where a position must be IGNORED. DistilBert wants this form.
    pub fn padding(&self) -> Result<Tensor> {
        let ones = self.mask.ones_like().map_err(mage_error::ai)?;
        (ones - &self.mask).map_err(mage_error::ai)
    }
}
