//! # orko-providers
//!
//! OpenAI-compatible provider engine and presets for
//! [orko](https://github.com/orko-rs/orko).
//!
//! - [`codec`] builds requests and parses the SSE stream. It does no IO.
//! - [`transport`] has the [`Transport`] trait for sending HTTP. The default,
//!   [`HyperTransport`], uses hyper and rustls and works under any async
//!   runtime. It is behind the `transport-hyper` feature, which is on by
//!   default.
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
//! If the default transport doesn't fit, pass your own [`Transport`] to
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
    pub(crate) mod openai;
}

pub use engine::{OpenAiCompatible, Quirks};
pub use providers::deepseek::{deepseek, DEEPSEEK_BASE_URL};
pub use providers::openai::{openai, OPENAI_BASE_URL};
#[cfg(feature = "transport-hyper")]
pub use transport::HyperTransport;
pub use transport::{ByteStream, HttpResponse, Transport};

use orko_core::{boxed, BoxProvider, Error, Result};

/// Builds a provider from a `"<provider>:<model>"` spec, e.g.
/// `"openai:gpt-4o-mini"` or `"deepseek:deepseek-chat"`.
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
        "openai" => Ok(boxed(openai(model))),
        "deepseek" => Ok(boxed(deepseek(model))),
        other => Err(Error::Config(format!("unknown provider `{other}`"))),
    }
}
