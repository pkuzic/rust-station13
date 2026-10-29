//! Versioned content definitions (prototypes) loaded from RON files.

use serde::de::DeserializeOwned;

/// Identifier of a content prototype, e.g. `"crowbar"`. Also the key used for its
/// localised noun in `rs13-i18n`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct ProtoId(pub String);

#[derive(Debug, thiserror::Error)]
pub enum ContentError {
    #[error("{path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: Box<ron::error::SpannedError>,
    },
}

/// Parse one RON content file. `path` is only used for error messages.
pub fn parse_ron<T: DeserializeOwned>(path: &str, text: &str) -> Result<T, ContentError> {
    ron::from_str(text).map_err(|source| ContentError::Parse {
        path: path.to_owned(),
        source: Box::new(source),
    })
}
