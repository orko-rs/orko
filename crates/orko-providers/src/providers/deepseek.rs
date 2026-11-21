//! The DeepSeek preset.

use crate::OpenAiCompatible;

/// DeepSeek's API base URL.
pub const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com/v1";

/// A DeepSeek provider using [`DEEPSEEK_BASE_URL`] and the key in `DEEPSEEK_API_KEY`.
///
/// The models are `deepseek-chat` and `deepseek-reasoner`. The reasoner's
/// thinking arrives in [`orko_core::CompletionChunk::reasoning`], and
/// [`orko_core::Agent::invoke`] returns only the final answer.
pub fn deepseek(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(DEEPSEEK_BASE_URL, model).with_api_key_env("DEEPSEEK_API_KEY")
}
