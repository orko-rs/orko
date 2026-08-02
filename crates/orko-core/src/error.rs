//! Error and result types shared across the toolkit.

use thiserror::Error;

/// The one error type surfaced by `orko`.
///
/// Public APIs return [`Result<T>`] — never `Result<(), String>`. Downstream
/// crates map their transport/protocol failures into these variants (usually
/// [`Error::Provider`]) so callers have a single error surface.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A provider (HTTP client, in-process engine, …) failed to produce a completion.
    #[error("provider error: {0}")]
    Provider(String),

    /// (De)serialization of a request/response/tool payload failed.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// A tool returned an error while executing.
    #[error("tool execution error: {0}")]
    ToolExecution(String),

    /// Invalid or missing configuration (bad model id, missing API key, …).
    #[error("configuration error: {0}")]
    Config(String),

    /// Anything that does not fit the categories above.
    #[error("{0}")]
    Other(String),
}

/// Crate-wide result alias.
pub type Result<T> = std::result::Result<T, Error>;
