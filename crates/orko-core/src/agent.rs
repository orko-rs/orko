//! The [`Agent`] and [`AgentBuilder`] types, and the [`create_agent`] entry point.

use crate::{
    CompletionOptions, CompletionRequest, CompletionStream, Error, Message, Prompt, Provider,
    Result, Tool,
};
use futures::StreamExt;
use std::sync::Arc;

/// Creates an agent around `provider`.
///
/// Generic over `T: Provider` and deliberately ignorant of any concrete
/// provider; accepts any type that implements [`Provider`]
pub fn create_agent<P: Provider>(provider: P) -> AgentBuilder<P> {
    AgentBuilder::new(provider)
}

/// Fluent and cloneable builder produced by [`create_agent`].
#[derive(Clone)]
pub struct AgentBuilder<P: Provider> {
    provider: P,
    model: Option<String>,
    system_prompt: Option<String>,
    tools: Vec<Arc<dyn Tool>>,
    max_tool_turns: usize,
}

impl<P: Provider> AgentBuilder<P> {
    fn new(provider: P) -> Self {
        Self {
            provider,
            model: None,
            system_prompt: None,
            tools: Vec::new(),
            max_tool_turns: MAX_TOOL_TURNS,
        }
    }

    /// Set the system prompt prepended to every conversation.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Overrides the model id sent with each request.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Attaches a set of tools the agent may call.
    pub fn with_tools(mut self, tools: impl IntoIterator<Item = Arc<dyn Tool>>) -> Self {
        self.tools.extend(tools);
        self
    }

    /// Attaches a single tool.
    pub fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    /// Caps the provider round-trips per [`Agent::invoke`] call.
    /// Defaults to [`MAX_TOOL_TURNS`]. A cap of `0` makes `invoke` always fail.
    pub fn with_max_tool_turns(mut self, turns: usize) -> Self {
        self.max_tool_turns = turns;
        self
    }

    /// Finalize into an [`Agent`].
    ///
    /// Sorts the tools by name; [`Agent::invoke`] binary-searches them when
    /// dispatching tool calls.
    pub fn build(mut self) -> Agent<P> {
        self.tools.sort_by(|a, b| a.name().cmp(b.name()));
        Agent {
            provider: self.provider,
            model: self.model,
            system_prompt: self.system_prompt,
            tools: self.tools,
            max_tool_turns: self.max_tool_turns,
        }
    }
}

/// A configured agent: a provider plus its system prompt and tools.
///
/// [`Agent::invoke`] runs the tool-calling loop: stream a completion, and if
/// the model requested tool calls, execute them, feed the results back, and
/// re-invoke — until the model answers in plain text.
/// [`Agent::invoke_stream`] is single-turn: it returns the raw first stream
/// and dispatches nothing.
pub struct Agent<P: Provider> {
    provider: P,
    model: Option<String>,
    system_prompt: Option<String>,
    tools: Vec<Arc<dyn Tool>>,
    max_tool_turns: usize,
}

impl<P: Provider> Agent<P> {
    /// The tools registered on this agent, sorted by name.
    pub fn tools(&self) -> &[Arc<dyn Tool>] {
        &self.tools
    }

    /// The system prompt (if any) followed by the prompt's messages.
    fn seed_messages(&self, prompt: Prompt) -> Vec<Message> {
        let mut messages = Vec::with_capacity(prompt.messages.len() + 1);
        if let Some(sp) = &self.system_prompt {
            messages.push(Message::system(sp.clone()));
        }
        messages.extend(prompt.messages);
        messages
    }

    fn options(&self) -> CompletionOptions {
        CompletionOptions {
            model: self.model.clone(),
            tools: self.tools.iter().map(|t| t.spec()).collect(),
            ..Default::default()
        }
    }

    /// Runs the agent to completion and return the final assistant reply.
    ///
    /// Streams a completion; when the model requests tool calls, each one is
    /// dispatched to the matching registered tool, the results are fed back
    /// into the conversation, and the provider is re-invoked. Returns the
    /// content of the first completion that requests no tool calls. A turn
    /// that streams only a refusal yields the refusal text instead of `""`.
    ///
    /// # Errors:
    ///
    /// Besides provider errors: a call to a tool that isn't registered is
    /// [`Error::ToolExecution`], unparseable call arguments are
    /// [`Error::Serialization`], and a conversation still requesting tools
    /// after the configured cap ([`MAX_TOOL_TURNS`] unless overridden with
    /// [`AgentBuilder::with_max_tool_turns`]) is [`Error::Other`].
    pub async fn invoke(&self, input: impl Into<Prompt>) -> Result<String> {
        let mut messages = self.seed_messages(input.into());
        for _ in 0..self.max_tool_turns {
            let request = CompletionRequest {
                messages: messages.clone(),
                options: self.options(),
            };
            let mut stream = self.provider.complete(request).await?;

            let mut content = String::new();
            let mut refusal = String::new();
            let mut calls: Vec<PendingCall> = Vec::new();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                content.push_str(&chunk.content);
                if let Some(r) = &chunk.refusal {
                    refusal.push_str(r);
                }
                for delta in chunk.tool_calls {
                    let i = delta.index as usize;
                    if calls.len() <= i {
                        calls.resize_with(i + 1, PendingCall::default);
                    }
                    if delta.id.is_some() {
                        calls[i].id = delta.id;
                    }
                    if delta.name.is_some() {
                        calls[i].name = delta.name;
                    }
                    calls[i].arguments.push_str(&delta.arguments);
                }
            }

            if calls.is_empty() {
                return Ok(if content.is_empty() { refusal } else { content });
            }

            messages.push(Message::assistant(render_assistant_turn(&content, &calls)));
            for call in calls {
                let name = call.name.as_deref().unwrap_or_default();
                let tool = self
                    .tools
                    .binary_search_by(|t| t.name().cmp(name))
                    .map(|i| &self.tools[i])
                    .map_err(|_| {
                        Error::ToolExecution(format!("model called unknown tool `{name}`"))
                    })?;
                let args = if call.arguments.trim().is_empty() {
                    serde_json::Value::Object(Default::default())
                } else {
                    serde_json::from_str(&call.arguments)?
                };
                let output = tool.call(args).await?;
                messages.push(Message::tool(match call.id.as_deref() {
                    Some(id) => format!("[{name}:{id}] {output}"),
                    None => format!("[{name}] {output}"),
                }));
            }
        }
        Err(Error::Other(format!(
            "tool-call loop still requesting tools after {} turns",
            self.max_tool_turns
        )))
    }

    /// Runs the agent and return the raw completion stream.
    ///
    /// Single-turn: tool calls in the stream are not dispatched, for full loop
    /// use [`invoke`](Self::invoke).
    pub async fn invoke_stream(&self, input: impl Into<Prompt>) -> Result<CompletionStream> {
        let request = CompletionRequest {
            messages: self.seed_messages(input.into()),
            options: self.options(),
        };
        self.provider.complete(request).await
    }
}

/// Default upper bound on provider round-trips per [`Agent::invoke`] call.
/// Override per agent with [`AgentBuilder::with_max_tool_turns`].
pub const MAX_TOOL_TURNS: usize = 8;

/// One tool call reassembled from streamed [`ToolCallDelta`](crate::ToolCallDelta) fragments.
#[derive(Default)]
struct PendingCall {
    id: Option<String>,
    name: Option<String>,
    arguments: String,
}

fn render_assistant_turn(content: &str, calls: &[PendingCall]) -> String {
    let mut out = String::from(content);
    for call in calls {
        if !out.is_empty() {
            out.push('\n');
        }
        let name = call.name.as_deref().unwrap_or("?");
        match &call.id {
            Some(id) => out.push_str(&format!("[tool_call:{id}] {name}({})", call.arguments)),
            None => out.push_str(&format!("[tool_call] {name}({})", call.arguments)),
        }
    }
    out
}
