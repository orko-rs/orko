//! # orko-core
//!
//! The small, dependency-light heart of the [orko](https://github.com/orko-rs/orko)
//! agent-orchestration toolkit. It defines the traits and value types every
//! other crate builds on and nothing else — no HTTP, no runtime, no transport.
//!
//! ## The one abstraction that matters: [`Provider`]
//!
//! A [`Provider`] maps messages to a stream of completion chunks. It is
//! deliberately HTTP-agnostic: an OpenAI-compatible HTTP client and an
//! in-process Candle engine implement the *same* trait. See [`Provider`]'s docs
//! for the generic-vs-erased design ([`BoxProvider`]).
//!
//! ## Stability
//!
//! Pre-1.0 (`0.1.x`): breaking changes are expected between minor versions.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(unreachable_pub)]

mod agent;
mod content;
mod error;
mod message;
mod provider;
mod router;
mod tool;

// TODO: RAG — a `retrieval` module with the core trait set, shaped like
// `provider`: `Document` (content + metadata), `Embedder` (text -> Vec<f32>),
// `Retriever` (query -> Vec<Document>). Traits live here, runtime-agnostic and
// HTTP-free; implementations (embedding endpoints, vector stores) belong in
// orko-providers or user crates.

pub use agent::{create_agent, Agent, AgentBuilder, MAX_TOOL_TURNS};
pub use content::{Content, ContentPart, ContentView, MediaSource};
pub use error::{Error, Result};
pub use message::{Message, Prompt, Role};
pub use provider::{
    boxed, BoxProvider, CompletionChunk, CompletionOptions, CompletionRequest, CompletionStream,
    DynProvider, FinishReason, Provider, ToolCallDelta, Usage,
};
pub use router::{ModelRouter, StaticRouter};
pub use tool::{Tool, ToolSpec};

// Not public API. Support for `#[orko_macros::tool]` expansions, which need a
// serde_json path that resolves in the user's crate without a direct dep.
#[doc(hidden)]
pub mod __private {
    pub use serde_json;
}

pub(crate) use tool::ToolRegistry;
