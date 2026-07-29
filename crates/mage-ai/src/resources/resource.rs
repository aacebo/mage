use std::path::PathBuf;

use super::format::Format;
use super::uri::Uri;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Resource {
    pub uri: Uri,
    pub format: Format,
    pub path: Option<PathBuf>,
    pub size: u64,
}

impl Resource {
    pub fn new(uri: Uri) -> Self {
        let format = uri.format();
        let path = match &uri {
            Uri::Local(path) => Some(path.clone()),
            Uri::Http(_) => None,
        };

        Self {
            uri,
            format,
            path,
            size: 0,
        }
    }

    pub fn name(&self) -> &str {
        self.uri.name().unwrap_or("??")
    }

    pub fn resolved(mut self, path: PathBuf, size: u64) -> Self {
        self.path = Some(path);
        self.size = size;
        self
    }
}

impl From<Uri> for Resource {
    fn from(uri: Uri) -> Self {
        Self::new(uri)
    }
}

impl std::fmt::Display for Resource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.uri)
    }
}
