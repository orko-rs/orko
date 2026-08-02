//! The [`Agent`] type, its [`AgentBuilder`], and the [`create_agent`] entry point.

use crate::{
    CompletionOptions, CompletionRequest, CompletionStream, Message, Prompt, Provider, Result, Tool,
};
use futures::StreamExt;
use std::sync::Arc;

/// Start building an agent around `provider`.
///
/// Generic over `T: Provider` and deliberately ignorant of any concrete
/// provider — you hand it something that implements [`Provider`] and it never
/// asks what it is.
pub fn create_agent<P: Provider>(provider: P) -> AgentBuilder<P> {
    AgentBuilder::new(provider)
}

/// Fluent builder produced by [`create_agent`].
pub struct AgentBuilder<P: Provider> {
    provider: P,
    tools: Vec<Arc<dyn Tool>>,
    system_prompt: Option<String>,
    model: Option<String>,
}

impl<P: Provider> AgentBuilder<P> {
    fn new(provider: P) -> Self {
        Self {
            provider,
            tools: Vec::new(),
            system_prompt: None,
            model: None,
        }
    }

    /// Attach a set of tools the agent may call.
    pub fn with_tools(mut self, tools: impl IntoIterator<Item = Arc<dyn Tool>>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// Attach a single tool.
    pub fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    /// Set the system prompt prepended to every conversation.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Override the model id sent with each request.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Finalize into an [`Agent`].
    pub fn build(self) -> Agent<P> {
        Agent {
            provider: self.provider,
            tools: self.tools,
            system_prompt: self.system_prompt,
            model: self.model,
        }
    }
}

/// A configured agent: a provider plus its tools and system prompt.
///
/// The tool-calling loop (dispatch a call, feed the result back, re-invoke) is
/// intentionally *not* implemented yet — [`Agent::invoke`] currently returns the
/// provider's first completion. The types are wired so adding that loop is a
/// local change here, not an API break.
pub struct Agent<P: Provider> {
    provider: P,
    tools: Vec<Arc<dyn Tool>>,
    system_prompt: Option<String>,
    model: Option<String>,
}

impl<P: Provider> Agent<P> {
    /// The tools registered on this agent.
    pub fn tools(&self) -> &[Arc<dyn Tool>] {
        &self.tools
    }

    fn build_request(&self, prompt: Prompt) -> CompletionRequest {
        let mut messages = Vec::with_capacity(prompt.messages.len() + 1);
        if let Some(sp) = &self.system_prompt {
            messages.push(Message::system(sp.clone()));
        }
        messages.extend(prompt.messages);
        let options = CompletionOptions {
            model: self.model.clone(),
            tools: self.tools.iter().map(|t| t.spec()).collect(),
            ..Default::default()
        };
        CompletionRequest { messages, options }
    }

    /// Run the agent to completion and return the full assistant reply.
    ///
    /// Drains [`invoke_stream`](Self::invoke_stream) and concatenates the
    /// chunks.
    pub async fn invoke(&self, input: impl Into<Prompt>) -> Result<String> {
        let mut stream = self.invoke_stream(input).await?;
        let mut out = String::new();
        while let Some(chunk) = stream.next().await {
            out.push_str(&chunk?.content);
        }
        Ok(out)
    }

    /// Run the agent and return the raw completion stream.
    pub async fn invoke_stream(&self, input: impl Into<Prompt>) -> Result<CompletionStream> {
        let request = self.build_request(input.into());
        self.provider.complete(request).await
    }
}
