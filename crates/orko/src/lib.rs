//! # orko
//!
//! Batteries-included facade for the [orko](https://github.com/orko-rs/orko)
//! agent-orchestration toolkit. Everything from `orko-core` is re-exported at
//! the root; the other crates sit behind cargo features:
//!
//! | Feature | Brings in | Default |
//! |---|---|---|
//! | `providers` | `orko::providers` (OpenAI-compatible engines, presets) | yes |
//! | `macros` | the `#[tool]` attribute macro | no |
//! | `graph` | `orko::graph` (event graphs) | no |
//! | `mcp` | `orko::mcp` (MCP client/server) | no |
//! | `runtime` | `orko::runtime` (Tokio-backed execution; implies `graph`) | no |
//! | `full` | all of the above | no |
//!
//! ## Building an agent
//!
//! ```no_run
//! use orko::{create_agent, Provider};
//!
//! # async fn demo(p: impl Provider) -> orko::Result<()> {
//! let agent = create_agent(p)
//!     .with_system_prompt("You are an agent orchestrated with Orko.")
//!     .build()?;
//! let reply = agent.invoke("Hello!").await?;
//! println!("{reply}");
//! # Ok(())
//! # }
//! ```
//!
//! ## Stability
//!
//! Pre-1.0 (`0.1.x`): breaking changes are expected between minor versions.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(unreachable_pub)]

pub use orko_core::*;

// The gated crates are still empty placeholders; the allows silence the
// nothing-to-re-export warnings and MUST be dropped as each crate gains items.

// TODO: narrow to `pub use orko_macros::tool;` once the #[tool] macro lands.
#[cfg(feature = "macros")]
#[allow(unused_imports, unreachable_pub)]
pub use orko_macros::*;

/// Provider implementations: OpenAI-compatible engine and presets.
#[cfg(feature = "providers")]
#[allow(unused_imports, unreachable_pub)]
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
