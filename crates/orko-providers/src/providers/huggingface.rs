//! The Hugging Face router preset.

use crate::OpenAiCompatible;

/// Hugging Face router's API base URL.
pub const HUGGINGFACE_BASE_URL: &str = "https://router.huggingface.co/v1";

/// A Hugging Face router provider using [`HUGGINGFACE_BASE_URL`] and the key in `HF_TOKEN`.
pub fn huggingface(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(HUGGINGFACE_BASE_URL, model).with_api_key_env("HF_TOKEN")
}
