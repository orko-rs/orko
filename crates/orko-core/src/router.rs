//! The [`ModelRouter`] trait: pick a model/provider for a given request.
//!
//! Only the trait and a trivial [`StaticRouter`] exist today. Cost-, latency-,
//! and capability-aware routing (the interesting part) is deferred; the trait is
//! here so that logic has a stable seam to slot into.

use crate::CompletionRequest;

/// Selects which model should serve a request.
///
/// Returns a model id string; wiring the chosen id back onto the request (and,
/// later, selecting among multiple providers) is the caller's job for now.
pub trait ModelRouter: Send + Sync {
    /// Choose a model id for `request`.
    fn route(&self, request: &CompletionRequest) -> String;
}

/// A router that always returns the same model, ignoring the request.
pub struct StaticRouter {
    model: String,
}

impl StaticRouter {
    /// Create a router pinned to `model`.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
        }
    }
}

impl ModelRouter for StaticRouter {
    fn route(&self, _request: &CompletionRequest) -> String {
        self.model.clone()
    }
}
