//! The Together AI preset.

use crate::OpenAiCompatible;

/// Together AI's API base URL.
pub const TOGETHER_BASE_URL: &str = "https://api.together.xyz/v1";

/// A Together AI provider using [`TOGETHER_BASE_URL`] and the key in `TOGETHER_API_KEY`.
pub fn together(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(TOGETHER_BASE_URL, model).with_api_key_env("TOGETHER_API_KEY")
}
