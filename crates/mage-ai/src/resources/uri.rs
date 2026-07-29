use std::path::PathBuf;
use std::str::FromStr;

use mage_error::{Error, Result};

use super::format::Format;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Uri {
    Local(PathBuf),
    Http(url::Url),
}

impl Uri {
    pub fn local(path: impl Into<PathBuf>) -> Self {
        Self::Local(path.into())
    }

    pub fn http(url: url::Url) -> Self {
        Self::Http(url)
    }

    pub fn parse(uri: &str) -> Result<Self> {
        let Some((scheme, rest)) = uri.split_once("://") else {
            // A bare path is local: `./models/bart`, `/abs/path`.
            return match uri.is_empty() {
                true => Err(mage_error::parse("empty uri")),
                false => Ok(Self::local(uri)),
            };
        };

        match scheme {
            "file" => Ok(Self::local(rest)),
            "http" | "https" => url::Url::parse(uri).map(Self::Http).map_err(mage_error::parse),
            _ => Err(mage_error::parse(format!("unsupported uri scheme: {scheme:?}"))),
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Local(path) => path.file_name()?.to_str(),
            Self::Http(url) => url.path_segments()?.next_back(),
        }
    }

    pub fn ext(&self) -> Option<&str> {
        let (_, ext) = self.name()?.rsplit_once('.')?;
        Some(ext)
    }

    pub fn format(&self) -> Format {
        self.ext().map(Format::from_ext).unwrap_or_default()
    }

    pub fn join(&self, file: &str) -> Result<Self> {
        match self {
            Self::Local(dir) => Ok(Self::Local(dir.join(file))),
            Self::Http(base) => {
                let base = match base.as_str().ends_with('/') {
                    true => base.clone(),
                    false => url::Url::parse(&format!("{base}/"))?,
                };

                base.join(file).map(Self::Http).map_err(mage_error::parse)
            }
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local(_))
    }
}

impl FromStr for Uri {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::parse(value)
    }
}

impl std::fmt::Display for Uri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Local(path) => write!(f, "file://{}", path.display()),
            Self::Http(url) => write!(f, "{url}"),
        }
    }
}

impl std::fmt::Debug for Uri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

impl serde::Serialize for Uri {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Uri {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::from_str(&value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_bare_path_as_local() {
        assert_eq!("./models/bart".parse::<Uri>().unwrap(), Uri::local("./models/bart"));
        assert_eq!("/abs/path".parse::<Uri>().unwrap(), Uri::local("/abs/path"));
    }

    #[test]
    fn parses_schemes() {
        assert!("file:///models/bart".parse::<Uri>().unwrap().is_local());
        assert!(!"https://host/models".parse::<Uri>().unwrap().is_local());
    }

    #[test]
    fn rejects_an_unsupported_scheme() {
        assert!("ftp://host/x".parse::<Uri>().is_err());
        assert!("".parse::<Uri>().is_err());
    }

    #[test]
    fn derives_the_format_from_the_extension() {
        assert_eq!(Uri::local("a/config.json").format(), Format::Json);
        assert_eq!(Uri::local("a/model.safetensors").format(), Format::SafeTensors);
        assert_eq!(Uri::local("a/vocab.txt").format(), Format::Text);
        assert_eq!(Uri::local("a/thing").format(), Format::Unknown);
    }

    /// Joining must not swallow the last path segment, which is what `Url::join` does when the
    /// base has no trailing slash.
    #[test]
    fn joins_a_file_onto_a_base() {
        let local = Uri::local("/models/bart").join("config.json").unwrap();
        assert_eq!(local, Uri::local("/models/bart/config.json"));

        let http = "https://host/models/bart"
            .parse::<Uri>()
            .unwrap()
            .join("config.json")
            .unwrap();
        assert_eq!(http.to_string(), "https://host/models/bart/config.json");
    }

    #[test]
    fn round_trips_through_serde() {
        let uri: Uri = "https://host/models/bart".parse().unwrap();
        let json = serde_json::to_string(&uri).unwrap();

        assert_eq!(serde_json::from_str::<Uri>(&json).unwrap(), uri);
    }
}
