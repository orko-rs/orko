//! The llama.cpp preset.

use crate::OpenAiCompatible;

/// llama.cpp's API base URL.
pub const LLAMACPP_BASE_URL: &str = "http://localhost:8080/v1";

/// A llama.cpp provider using [`LLAMACPP_BASE_URL`] and the key in `LLAMACPP_API_KEY`.
///
/// Talks to a local `llama-server` on the default port. The key is optional
/// and only needed if the server was started with `--api-key`.
pub fn llamacpp(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(LLAMACPP_BASE_URL, model).with_api_key_env("LLAMACPP_API_KEY")
}
