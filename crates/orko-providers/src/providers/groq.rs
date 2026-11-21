//! The Groq preset.

use crate::OpenAiCompatible;

/// Groq's API base URL.
pub const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";

/// A Groq provider using [`GROQ_BASE_URL`] and the key in `GROQ_API_KEY`.
pub fn groq(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(GROQ_BASE_URL, model).with_api_key_env("GROQ_API_KEY")
}
