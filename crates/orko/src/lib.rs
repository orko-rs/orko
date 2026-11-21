//! # orko
//!
//! The batteries-included facade for the [orko](https://github.com/orko-rs/orko)
//! agent-orchestration toolkit. Everything from `orko-core` is re-exported at
//! the root; the other crates sit behind cargo features:
//!
//! | Feature | Brings in | Default |
//! |---|---|---|
//! | `providers` | `orko::providers` (OpenAI-compatible providers: `openai`, `deepseek`, `from_str`) | yes |
//! | `macros` | the `#[tool]` attribute macro | no |
//! | `graph` | `orko::graph` (event graphs) | no |
//! | `mcp` | `orko::mcp` (MCP client/server) | no |
//! | `runtime` | `orko::runtime` (Tokio-backed execution; implies `graph`) | no |
//! | `full` | all of the above | no |
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

// graph, mcp and runtime are still empty placeholders.

#[cfg(feature = "macros")]
pub use orko_macros::tool;

/// OpenAI-compatible providers and their presets (`openai`, `deepseek`).
#[cfg(feature = "providers")]
pub mod providers {
    pub use orko_providers::*;
}

/// Event-graph building blocks.
#[cfg(feature = "graph")]
#[allow(unused_imports, unreachable_pub)]
pub mod graph {
    pub use orko_graph::*;
}

/// Model Context Protocol client and server.
#[cfg(feature = "mcp")]
#[allow(unused_imports, unreachable_pub)]
pub mod mcp {
    pub use orko_mcp::*;
}

/// Tokio-backed execution: supervision, graph running, checkpointing.
#[cfg(feature = "runtime")]
#[allow(unused_imports, unreachable_pub)]
pub mod runtime {
    pub use orko_runtime::*;
}
