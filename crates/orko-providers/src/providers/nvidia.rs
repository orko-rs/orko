//! The NVIDIA NIM preset.

use crate::OpenAiCompatible;

/// NVIDIA NIM's API base URL.
pub const NVIDIA_BASE_URL: &str = "https://integrate.api.nvidia.com/v1";

/// A NVIDIA NIM provider using [`NVIDIA_BASE_URL`] and the key in `NVIDIA_API_KEY`.
///
/// For a self-hosted NIM, use `OpenAiCompatible::new` with its URL.
pub fn nvidia(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(NVIDIA_BASE_URL, model).with_api_key_env("NVIDIA_API_KEY")
}
