//! The [`Provider`] trait is central abstraction for providers.
//!
//! A `Provider` turns messages into a stream of completion chunks in a runtime agnostic way.
//! That is the *entire* contract. Any transport (HTTP, in-process, etc.) is handled by the caller.

use crate::tool::ToolSpec;
use crate::{Message, Result};
use futures::Stream;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Per-request knobs. All optional; a provider falls back to its defaults if not set.
#[derive(Debug, Clone, Default)]
pub struct CompletionOptions {
    /// Model identifier, e.g. `"gpt-4o-mini"`. `None` means "provider default".
    pub model: Option<String>,
    /// Sampling temperature. Range: [0.0, 2.0].
    pub temperature: Option<f32>,
    /// Maximum tokens to generate.
    pub max_tokens: Option<u32>,
    /// Tools the model may call this turn.
    pub tools: Vec<ToolSpec>,
}

/// A single completion request: the conversation and its options.
#[derive(Debug, Clone)]
pub struct CompletionRequest {
    /// The full message history to complete against.
    pub messages: Vec<Message>,
    /// Request options.
    pub options: CompletionOptions,
}

impl CompletionRequest {
    /// Build a request from messages with default options.
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            options: CompletionOptions::default(),
        }
    }

    /// Set the request options, returning `self` for chaining:
    /// `CompletionRequest::new(msgs).with_options(opts)`.
    pub fn with_options(mut self, options: CompletionOptions) -> Self {
        self.options = options;
        self
    }
}

/// One incremental piece of a streamed completion.
///
/// Construct with functional-update syntax so future field additions stay
/// painless: `CompletionChunk { content: "hi".into(), ..Default::default() }`.
#[derive(Debug, Clone, Default)]
pub struct CompletionChunk {
    /// The text delta carried by this chunk (may be empty).
    pub content: String,
    /// Reasoning/thinking delta
    pub reasoning: Option<String>,
    /// Refusal-text delta; some providers stream refusals in a dedicated field.
    pub refusal: Option<String>,
    /// Tool-call fragments carried by this chunk.
    pub tool_calls: Vec<ToolCallDelta>,
    /// Present on the final chunk: why generation stopped.
    pub finish_reason: Option<FinishReason>,
    /// Present on the final chunk if the provider reports token usage.
    pub usage: Option<Usage>,
}

/// A fragment of a streamed tool call.
///
/// `id` and `name` typically arrive once, in the first fragment; `arguments`
/// is a JSON string streamed in pieces. Consumers accumulate fragments by
/// `index` until the stream ends, then parse the assembled JSON.
#[derive(Debug, Clone, Default)]
pub struct ToolCallDelta {
    /// Which concurrent tool call this fragment belongs to.
    pub index: u32,
    /// Provider-assigned call id (first fragment only).
    pub id: Option<String>,
    /// Tool name (first fragment only).
    pub name: Option<String>,
    /// Incremental JSON fragment of the arguments.
    pub arguments: String,
}

/// Why the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FinishReason {
    /// Natural end of the response.
    Stop,
    /// Hit the `max_tokens` limit — output is truncated.
    Length,
    /// The model wants tool results before continuing.
    ToolCalls,
    /// The provider filtered the content.
    ContentFilter,
}

/// Token counts for a completion, usually reported on the final chunk.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    /// Tokens in the prompt.
    pub prompt_tokens: u32,
    /// Tokens generated.
    pub completion_tokens: u32,
}

/// A thread-safe, transport-agnostic, runtime-agnostic async stream of independently
/// fallible completion chunks.
pub type CompletionStream = Pin<Box<dyn Stream<Item = Result<CompletionChunk>> + Send>>;

/// # Object safety
///
/// This trait is **generic-first**: [`Agent`](crate::Agent) is `Agent<P: Provider>`,
/// so the common path is monomorphized and allocation-free. `async fn` in
/// traits is not object-safe, so for the cases that need type erasure (a
/// heterogeneous registry, `from_str("openai:gpt-4o")`, dynamic routing) we
/// provide [`BoxProvider`] via the [`DynProvider`] shim below.
pub trait Provider: Send + Sync {
    /// Produce a streaming completion for `request`.
    ///
    /// Returns `impl Future` (rather than `async fn`) with an explicit `Send`
    /// bound so the returned future can cross threads on any executor.
    fn complete(
        &self,
        request: CompletionRequest,
    ) -> impl Future<Output = Result<CompletionStream>> + Send;
}

/// `Arc<P>` is itself a [`Provider`], delegating to the inner value. This makes
/// any provider cheaply cloneable.
impl<P: Provider> Provider for Arc<P> {
    fn complete(
        &self,
        request: CompletionRequest,
    ) -> impl Future<Output = Result<CompletionStream>> + Send {
        (**self).complete(request)
    }
}

/// Object-safe sibling of [`Provider`]. It exists so `Box<dyn DynProvider>` can
/// be a [`Provider`]. The blanket impl erases any concrete `Provider` into it by
/// boxing the returned future.
pub trait DynProvider: Send + Sync {
    /// Boxed-future form of [`Provider::complete`].
    fn complete_dyn(
        &self,
        request: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<CompletionStream>> + Send + '_>>;
}

impl<P: Provider> DynProvider for P {
    fn complete_dyn(
        &self,
        request: CompletionRequest,
    ) -> Pin<Box<dyn Future<Output = Result<CompletionStream>> + Send + '_>> {
        Box::pin(self.complete(request))
    }
}

/// A type-erased provider. `BoxProvider` is itself a [`Provider`], so it drops
/// straight into `Agent<BoxProvider>` and any `T: Provider` bound.
pub type BoxProvider = Box<dyn DynProvider>;

impl Provider for BoxProvider {
    fn complete(
        &self,
        request: CompletionRequest,
    ) -> impl Future<Output = Result<CompletionStream>> + Send {
        (**self).complete_dyn(request)
    }
}

/// Erase a concrete [`Provider`] into a [`BoxProvider`].
pub fn boxed<P: Provider + 'static>(provider: P) -> BoxProvider {
    Box::new(provider)
}
