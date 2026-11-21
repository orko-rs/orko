//! The Z.ai (GLM) preset.

use crate::OpenAiCompatible;

/// Z.ai (GLM)'s API base URL.
pub const ZAI_BASE_URL: &str = "https://api.z.ai/api/paas/v4";

/// A Z.ai (GLM) provider using [`ZAI_BASE_URL`] and the key in `ZAI_API_KEY`.
pub fn zai(model: impl Into<String>) -> OpenAiCompatible {
    OpenAiCompatible::new(ZAI_BASE_URL, model).with_api_key_env("ZAI_API_KEY")
}
