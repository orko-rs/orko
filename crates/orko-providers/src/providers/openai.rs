//! The OpenAI preset.

use crate::{OpenAiCompatible, Quirks};

/// OpenAI's API base URL.
pub const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";

/// An OpenAI provider using [`OPENAI_BASE_URL`] and the key in `OPENAI_API_KEY`.
pub fn openai(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(OPENAI_BASE_URL, model)
        .with_api_key_env("OPENAI_API_KEY")
        .with_quirks(Quirks {
            max_tokens_field: "max_completion_tokens",
            ..Quirks::default()
        })
}
