//! The Google Gemini preset.

use crate::OpenAiCompatible;

/// Google Gemini's API base URL.
pub const GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/openai";

/// A Google Gemini provider using [`GEMINI_BASE_URL`] and the key in `GEMINI_API_KEY`.
///
/// Uses Gemini's OpenAI-compatible endpoint, e.g. `gemini("gemini-2.5-flash")`.
pub fn gemini(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(GEMINI_BASE_URL, model).with_api_key_env("GEMINI_API_KEY")
}
