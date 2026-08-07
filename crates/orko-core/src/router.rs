//! The [`ModelRouter`] trait: pick a model/provider for a given request.
//!
//! Only the trait and a trivial [`StaticRouter`] exist today. Cost-, latency-,
//! and capability-aware routing is deferred; the trait is here so that logic
//! has a stable seam to slot into.

use crate::CompletionRequest;

// TODO: multi-model / multi-provider agents:
// - Shape: a `MultiProvider` composite that implements `Provider` — holds
//   `Vec<(String, BoxProvider)>` members plus a `ModelRouter`, rewrites
//   `request.model` and delegates. Agent stays `Agent<P>` single-provider-
//   shaped forever; multiplicity lives inside the composite.
// - Routing coordinate: the same `"provider:model"` string the orko-providers
//   presets' `from_str` will use — one coordinate system everywhere, so the
//   `String` return of `route()` may already suffice.
// - Fallback-on-provider-failure is policy inside the composite (e.g. retry
//   another member on 5xx); the invoke loop's "provider errors are hard"
//   contract is unchanged.

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
