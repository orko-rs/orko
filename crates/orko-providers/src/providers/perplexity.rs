//! The Perplexity preset.

use crate::{OpenAiCompatible, Quirks};

/// Perplexity's API base URL.
pub const PERPLEXITY_BASE_URL: &str = "https://api.perplexity.ai";

/// A Perplexity provider using [`PERPLEXITY_BASE_URL`] and the key in `PERPLEXITY_API_KEY`.
///
/// Perplexity sends usage on its own, so `stream_options` is off.
pub fn perplexity(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(PERPLEXITY_BASE_URL, model)
        .with_api_key_env("PERPLEXITY_API_KEY")
        .with_quirks(Quirks {
            stream_usage: false,
            ..Quirks::default()
        })
}
