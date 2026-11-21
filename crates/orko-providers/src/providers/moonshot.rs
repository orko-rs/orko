//! The Moonshot (Kimi) preset.

use crate::OpenAiCompatible;

/// Moonshot (Kimi)'s API base URL.
pub const MOONSHOT_BASE_URL: &str = "https://api.moonshot.ai/v1";

/// A Moonshot (Kimi) provider using [`MOONSHOT_BASE_URL`] and the key in `MOONSHOT_API_KEY`.
pub fn moonshot(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(MOONSHOT_BASE_URL, model).with_api_key_env("MOONSHOT_API_KEY")
}
