//! The Ollama preset.

use crate::OpenAiCompatible;

/// Ollama's API base URL.
pub const OLLAMA_BASE_URL: &str = "http://localhost:11434/v1";

/// An Ollama provider using [`OLLAMA_BASE_URL`]. It sends no API key.
///
/// Talks to a local Ollama server on the default port.
pub fn ollama(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(OLLAMA_BASE_URL, model)
}
