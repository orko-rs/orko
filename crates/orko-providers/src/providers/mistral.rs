//! The Mistral preset.

use crate::{OpenAiCompatible, Quirks};

/// Mistral's API base URL.
pub const MISTRAL_BASE_URL: &str = "https://api.mistral.ai/v1";

/// A Mistral provider using [`MISTRAL_BASE_URL`] and the key in `MISTRAL_API_KEY`.
///
/// Mistral rejects `stream_options`, so it's off. Usage still arrives in the last chunk.
pub fn mistral(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(MISTRAL_BASE_URL, model)
        .with_api_key_env("MISTRAL_API_KEY")
        .with_quirks(Quirks {
            stream_usage: false,
            ..Quirks::default()
        })
}
