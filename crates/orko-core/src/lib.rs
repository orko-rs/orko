//! # orko-core
//!
//! The frozen, dependency-light heart of the [orko](https://github.com/orko-rs/orko)
//! agent-orchestration toolkit. It defines the traits and value types every
//! other crate builds on and nothing else — no HTTP, no runtime, no transport.
//!
//! ## The one abstraction that matters: [`Provider`]
//!
//! A [`Provider`] maps messages to a stream of completion chunks. It is
//! deliberately HTTP-agnostic: an OpenAI-compatible HTTP client and an
//! in-process Candle engine implement the *same* trait. See [`provider`] for the
//! generic-vs-erased design ([`BoxProvider`]).
//!
//! ## Building an agent
//!
//! ```no_run
//! use orko_core::{create_agent, Provider};
//!
//! # async fn demo(p: impl Provider) -> orko_core::Result<()> {
//! let agent = create_agent(p)
//!     .with_system_prompt("You are a helpful assistant.")
//!     .build();
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

mod agent;
mod error;
mod message;
mod provider;
mod router;
mod tool;

pub use agent::{create_agent, Agent, AgentBuilder};
pub use error::{Error, Result};
pub use message::{Message, Prompt, Role};
pub use provider::{
    boxed, BoxProvider, CompletionChunk, CompletionOptions, CompletionRequest, CompletionStream,
    DynProvider, FinishReason, Provider, ToolCallDelta, Usage,
};
pub use router::{ModelRouter, StaticRouter};
pub use tool::{Tool, ToolSpec};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_roundtrips_through_json() {
        let msg = Message::user("hello");
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"role":"user","content":"hello"}"#);
        let back: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(back, msg);
    }

    #[test]
    fn prompt_from_str_wraps_a_user_message() {
        let prompt: Prompt = "hi".into();
        assert_eq!(prompt.messages.len(), 1);
        assert_eq!(prompt.messages[0].role, Role::User);
    }

    #[test]
    fn chunk_functional_update_construction() {
        let last = CompletionChunk {
            finish_reason: Some(FinishReason::ToolCalls),
            ..Default::default()
        };
        assert!(last.content.is_empty());
        assert!(last.tool_calls.is_empty());
        assert_eq!(last.finish_reason, Some(FinishReason::ToolCalls));
    }

    #[test]
    fn static_router_ignores_request() {
        let router = StaticRouter::new("gpt-4o-mini");
        let req = CompletionRequest::new(vec![Message::user("x")]);
        assert_eq!(router.route(&req), "gpt-4o-mini");
    }
}
