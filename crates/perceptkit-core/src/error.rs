//! Unified error type for perceptkit-core.

use std::path::PathBuf;

use thiserror::Error;

/// Top-level error type.
#[derive(Debug, Error)]
pub enum Error {
    /// I/O error (e.g. reading scene files).
    #[error("io error on {path:?}: {source}")]
    Io {
        /// File path where the I/O error occurred, if known.
        path: Option<PathBuf>,
        /// Underlying std I/O error.
        #[source]
        source: std::io::Error,
    },

    /// YAML parse error with location hint.
    #[error("yaml parse error in {path:?}: {message}")]
    YamlParse {
        /// File path of the malformed YAML.
        path: PathBuf,
        /// Parser message with line/column hints.
        message: String,
    },

    /// Unknown feature referenced in a scene YAML — did_you_mean suggestion.
    #[error("unknown feature '{key}' in scene {scene_id}{}",
        did_you_mean.as_deref().map(|s| format!(", did you mean '{s}'?")).unwrap_or_default())]
    UnknownFeature {
        /// Feature key that didn't resolve.
        key: String,
        /// Scene id that referenced the unknown key.
        scene_id: String,
        /// Closest known key by Levenshtein distance, if any.
        did_you_mean: Option<String>,
    },

    /// Invalid feature key format (must be `a.b.c` dot-segments).
    #[error("invalid feature key '{0}': keys must be dot-segmented alphanumeric (e.g. 'audio.voice_ratio')")]
    InvalidFeatureKey(String),

    /// Scene id collision.
    #[error("duplicate scene id '{0}'")]
    DuplicateScene(String),

    /// Scene DSL schema violation.
    #[error("invalid scene '{scene_id}': {message}")]
    InvalidScene {
        /// Scene id with the problem.
        scene_id: String,
        /// Human-readable reason.
        message: String,
    },

    /// SQLite error from the PendingSceneQueue.
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// Configuration error.
    #[error("config error: {0}")]
    Config(String),
}

/// Convenience `Result` alias.
pub type Result<T> = std::result::Result<T, Error>;

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Self::Io {
            path: None,
            source,
        }
    }
}
