//! # orko
//!
//! The batteries-included facade for the [orko](https://github.com/orko-rs/orko)
//! agent-orchestration toolkit. Everything from `orko-core` is re-exported at
//! the root; the other crates sit behind cargo features:
//!
//! | Feature | Brings in | Default |
//! |---|---|---|
//! | `providers` | `orko::providers` (OpenAI-compatible presets such as `openai`, `deepseek`, `ollama`, plus `from_str`) | yes |
//! | `macros` | the `#[tool]` attribute macro | no |
//! | `full` | all of the above | no |
//!
//! Event graphs, MCP and the runtime are planned. Their features will come
//! back once those crates have code.
//!
//! ## Building an agent
//!
//! ```no_run
//! # #[cfg(feature = "providers")]
//! # async fn demo() -> orko::Result<()> {
//! use orko::{create_agent, providers::deepseek};
//!
//! // Reads the key from DEEPSEEK_API_KEY.
//! let agent = create_agent(deepseek("deepseek-chat"))
//!     .with_system_prompt("You are an agent orchestrated with Orko.")
//!     .build()?;
//! let reply = agent.invoke("Hello!").await?;
//! println!("{reply}");
//! # Ok(())
//! # }
//! ```
//!
//! The built-in transport runs on Tokio. For another runtime, pass your own
//! `Transport` to `OpenAiCompatible::with_transport`. To pick the provider at run time, use
//! `orko::providers::from_str("openai:gpt-4o-mini")`.
//!
//! ## Stability
//!
//! Pre-1.0 (`0.1.x`): breaking changes are expected between minor versions.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(unreachable_pub)]

pub use orko_core::*;

#[cfg(feature = "macros")]
pub use orko_macros::tool;

/// OpenAI-compatible providers and their presets (`openai`, `deepseek`, `groq`, `ollama` and more).
#[cfg(feature = "providers")]
pub mod providers {
    pub use orko_providers::*;
}
