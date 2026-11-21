//! The Qwen (DashScope) preset.

use crate::OpenAiCompatible;

/// Qwen (DashScope)'s API base URL.
pub const QWEN_BASE_URL: &str = "https://dashscope-intl.aliyuncs.com/compatible-mode/v1";

/// A Qwen (DashScope) provider using [`QWEN_BASE_URL`] and the key in `DASHSCOPE_API_KEY`.
///
/// This is the international endpoint. For the mainland China one, use
/// `OpenAiCompatible::new("https://dashscope.aliyuncs.com/compatible-mode/v1", model)`.
pub fn qwen(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(QWEN_BASE_URL, model).with_api_key_env("DASHSCOPE_API_KEY")
}
