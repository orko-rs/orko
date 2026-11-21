//! The OpenRouter preset.

use crate::OpenAiCompatible;

/// OpenRouter's API base URL.
pub const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

/// A OpenRouter provider using [`OPENROUTER_BASE_URL`] and the key in `OPENROUTER_API_KEY`.
///
/// Model ids include the vendor, e.g. `openrouter("openai/gpt-4o-mini")`.
pub fn openrouter(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(OPENROUTER_BASE_URL, model).with_api_key_env("OPENROUTER_API_KEY")
}
