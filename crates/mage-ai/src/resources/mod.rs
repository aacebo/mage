mod asset;
pub mod cache;
mod format;
mod resource;
mod uri;

use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use asset::{Asset, AssetData, Directory as AssetDirectory, File as AssetFile};
pub use format::Format;
use mage_error::Result;
pub use resource::Resource;
pub use uri::Uri;

pub trait Repository: Send + Sync {
    fn exists(&self, path: &Path) -> bool;
    fn get(&self, path: &Path) -> Result<Asset>;
    fn read(&self, path: &Path) -> Result<AssetData>;
    fn resolve(&self, path: &Path) -> Result<PathBuf>;
}

pub trait DataSource: Send + Sync {
    fn load(&self, key: &str) -> Result<Arc<dyn Repository>>;
}
