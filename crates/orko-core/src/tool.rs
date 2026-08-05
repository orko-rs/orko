//! The [`Tool`] trait: a capability the model can invoke by name.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// The wire description of a tool, handed to a provider so the model knows the
/// tool exists and how to call it.
///
/// This is the JSON-schema-bearing struct that [`orko-macros`](../orko_macros)
/// generates from a function signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    /// Unique tool name (usually the source function name).
    pub name: String,
    /// Human/model-readable description (usually the function's doc comment).
    pub description: String,
    /// JSON Schema describing the tool's parameters (an `object` schema).
    pub parameters: serde_json::Value,
}

/// A capability the agent can call while answering.
///
/// Object-safe on purpose: agents hold `Arc<dyn Tool>` so tools of different
/// concrete types can live in one collection. Because it must be object-safe,
/// [`Tool::call`] returns a boxed future rather than using `async fn` (which is
/// not yet object-safe in traits). No Tokio types appear here — the future is a
/// plain `std::future::Future`.
pub trait Tool: Send + Sync {
    /// The tool's unique name.
    fn name(&self) -> &str;

    /// A description of what the tool does; shown to the model.
    fn description(&self) -> &str;

    /// JSON Schema for the tool's parameters (an `object` schema).
    fn parameters_schema(&self) -> serde_json::Value;

    /// Execute the tool against JSON `args` and return its textual result.
    ///
    /// Implementors deserialize `args` into their expected parameter struct;
    /// a shape mismatch should surface as [`Error::ToolExecution`](crate::Error::ToolExecution).
    fn call<'a>(
        &'a self,
        args: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>>;

    /// Build the [`ToolSpec`] sent to providers. Defaulted from the accessors.
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name().to_owned(),
            description: self.description().to_owned(),
            parameters: self.parameters_schema(),
        }
    }
}

/// An agent's tool set, indexed by name.
///
/// Owns the invariants the lookup relies on: the tools are sorted by name and
/// names are unique.
pub struct ToolRegistry(Vec<Arc<dyn Tool>>);

impl ToolRegistry {
    /// Build a registry from `tools`, sorting them by name.
    ///
    /// # Errors
    ///
    /// [`Error::Config`] if two tools share a name.
    pub fn new(tools: impl IntoIterator<Item = Arc<dyn Tool>>) -> Result<Self> {
        let mut tools: Vec<_> = tools.into_iter().collect();
        tools.sort_by(|a, b| a.name().cmp(b.name()));
        if let Some(pair) = tools.windows(2).find(|w| w[0].name() == w[1].name()) {
            return Err(Error::Config(format!(
                "duplicate tool name `{}`",
                pair[0].name()
            )));
        }
        Ok(Self(tools))
    }

    /// Looks up a tool by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.0
            .binary_search_by(|t| t.name().cmp(name))
            .ok()
            .map(|i| &self.0[i])
    }

    /// The tools, sorted by name.
    pub fn as_slice(&self) -> &[Arc<dyn Tool>] {
        &self.0
    }
}
