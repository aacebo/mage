use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Asset {
    File(File),
    Directory(Directory),
}

impl Asset {
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self::File(File::new(path))
    }

    pub fn directory(path: impl Into<PathBuf>) -> Self {
        Self::Directory(Directory::new(path))
    }

    pub fn path(&self) -> &Path {
        match self {
            Self::File(file) => file.path(),
            Self::Directory(dir) => dir.path(),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::File(file) => file.name(),
            Self::Directory(dir) => dir.name(),
        }
    }

    pub fn ext(&self) -> Option<&str> {
        match self {
            Self::File(file) => file.ext(),
            Self::Directory(_) => None,
        }
    }
}

impl From<File> for Asset {
    fn from(file: File) -> Self {
        Self::File(file)
    }
}

impl From<Directory> for Asset {
    fn from(dir: Directory) -> Self {
        Self::Directory(dir)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssetData {
    File(Vec<u8>),
    Directory(Vec<Asset>),
}

impl AssetData {
    pub fn bytes(&self) -> Option<&[u8]> {
        match self {
            Self::File(bytes) => Some(bytes),
            Self::Directory(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct File {
    path: PathBuf,
    name: String,
    ext: Option<String>,
}

impl File {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let name = name_of(&path);
        let ext = path.extension().and_then(|ext| ext.to_str()).map(str::to_string);

        Self { path, name, ext }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ext(&self) -> Option<&str> {
        self.ext.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Directory {
    path: PathBuf,
    name: String,
}

impl Directory {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let name = name_of(&path);

        Self { path, name }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string()
}
