//! The xAI preset.

use crate::OpenAiCompatible;

/// xAI's API base URL.
pub const XAI_BASE_URL: &str = "https://api.x.ai/v1";

/// An xAI provider using [`XAI_BASE_URL`] and the key in `XAI_API_KEY`.
pub fn xai(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(XAI_BASE_URL, model).with_api_key_env("XAI_API_KEY")
}
