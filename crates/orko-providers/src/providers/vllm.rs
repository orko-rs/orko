//! The vLLM preset.

use crate::OpenAiCompatible;

/// vLLM's API base URL.
pub const VLLM_BASE_URL: &str = "http://localhost:8000/v1";

/// A vLLM provider using [`VLLM_BASE_URL`] and the key in `VLLM_API_KEY`.
///
/// Talks to a local vLLM server on the default port. The key is optional and
/// only needed if the server was started with `--api-key`.
pub fn vllm(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(VLLM_BASE_URL, model).with_api_key_env("VLLM_API_KEY")
}
