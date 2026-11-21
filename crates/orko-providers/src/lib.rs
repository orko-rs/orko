//! # orko-providers
//!
//! OpenAI-compatible provider engine and presets for
//! [orko](https://github.com/orko-rs/orko).
//!
//! - [`codec`] builds requests and parses the SSE stream. It does no IO.
//! - [`transport`] has the [`Transport`] trait for sending HTTP. The built-in
//!   [`TokioTransport`] runs hyper on Tokio, behind the `transport-tokio`
//!   feature, which is on by default.
//! - [`OpenAiCompatible`] puts the two together and implements
//!   [`orko_core::Provider`]. Presets such as [`openai`] set it up for one
//!   provider.
//!
//! ```no_run
//! # async fn demo() -> orko_core::Result<()> {
//! let agent = orko_core::create_agent(orko_providers::openai("gpt-4o-mini")).build()?;
//! println!("{}", agent.invoke("Hello!").await?);
//! # Ok(())
//! # }
//! ```
//!
//! For another runtime, or if the built-in transport doesn't fit, pass your own [`Transport`] to
//! [`OpenAiCompatible::with_transport`], or implement [`orko_core::Provider`]
//! yourself with any HTTP client.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(unreachable_pub)]

pub mod codec;
mod engine;
pub mod transport;

// Each preset has its own file in `src/providers/`. They are re-exported here.
mod providers {
    pub(crate) mod deepseek;
    pub(crate) mod gemini;
    pub(crate) mod groq;
    pub(crate) mod huggingface;
    pub(crate) mod llamacpp;
    pub(crate) mod mistral;
    pub(crate) mod moonshot;
    pub(crate) mod nvidia;
    pub(crate) mod ollama;
    pub(crate) mod openai;
    pub(crate) mod openrouter;
    pub(crate) mod perplexity;
    pub(crate) mod qwen;
    pub(crate) mod together;
    pub(crate) mod vllm;
    pub(crate) mod xai;
    pub(crate) mod zai;
}

pub use engine::{OpenAiCompatible, Quirks};
pub use providers::deepseek::{deepseek, DEEPSEEK_BASE_URL};
pub use providers::gemini::{gemini, GEMINI_BASE_URL};
pub use providers::groq::{groq, GROQ_BASE_URL};
pub use providers::huggingface::{huggingface, HUGGINGFACE_BASE_URL};
pub use providers::llamacpp::{llamacpp, LLAMACPP_BASE_URL};
pub use providers::mistral::{mistral, MISTRAL_BASE_URL};
pub use providers::moonshot::{moonshot, MOONSHOT_BASE_URL};
pub use providers::nvidia::{nvidia, NVIDIA_BASE_URL};
pub use providers::ollama::{ollama, OLLAMA_BASE_URL};
pub use providers::openai::{openai, OPENAI_BASE_URL};
pub use providers::openrouter::{openrouter, OPENROUTER_BASE_URL};
pub use providers::perplexity::{perplexity, PERPLEXITY_BASE_URL};
pub use providers::qwen::{qwen, QWEN_BASE_URL};
pub use providers::together::{together, TOGETHER_BASE_URL};
pub use providers::vllm::{vllm, VLLM_BASE_URL};
pub use providers::xai::{xai, XAI_BASE_URL};
pub use providers::zai::{zai, ZAI_BASE_URL};
#[cfg(feature = "transport-tokio")]
pub use transport::TokioTransport;
pub use transport::{ByteStream, HttpResponse, Transport};

use orko_core::{boxed, BoxProvider, Error, Result};

/// Builds a provider from a `"<provider>:<model>"` spec, e.g.
/// `"openai:gpt-4o-mini"` or `"ollama:llama3.2"`. The provider name is the
/// preset function's name, and everything after the first `:` is the model.
///
/// # Errors
///
/// [`Error::Config`] for a malformed spec or an unknown provider.
pub fn from_str(spec: &str) -> Result<BoxProvider> {
    let (provider, model) = spec
        .split_once(':')
        .filter(|(p, m)| !p.is_empty() && !m.is_empty())
        .ok_or_else(|| Error::Config(format!("expected `<provider>:<model>`, got `{spec}`")))?;
    match provider {
        "deepseek" => Ok(boxed(deepseek(model))),
        "gemini" => Ok(boxed(gemini(model))),
        "groq" => Ok(boxed(groq(model))),
        "huggingface" => Ok(boxed(huggingface(model))),
        "llamacpp" => Ok(boxed(llamacpp(model))),
        "mistral" => Ok(boxed(mistral(model))),
        "moonshot" => Ok(boxed(moonshot(model))),
        "nvidia" => Ok(boxed(nvidia(model))),
        "ollama" => Ok(boxed(ollama(model))),
        "openai" => Ok(boxed(openai(model))),
        "openrouter" => Ok(boxed(openrouter(model))),
        "perplexity" => Ok(boxed(perplexity(model))),
        "qwen" => Ok(boxed(qwen(model))),
        "together" => Ok(boxed(together(model))),
        "vllm" => Ok(boxed(vllm(model))),
        "xai" => Ok(boxed(xai(model))),
        "zai" => Ok(boxed(zai(model))),
        other => Err(Error::Config(format!("unknown provider `{other}`"))),
    }
}
