//! Conversation primitives: [`Role`], [`Message`], and the [`Prompt`] input type.

use serde::{Deserialize, Serialize};

/// The author of a [`Message`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System / developer instructions that steer the assistant.
    System,
    /// Input from the end user.
    User,
    /// Output from the model.
    Assistant,
    /// The result of a tool call, fed back into the conversation.
    Tool,
}

/// A single turn in a conversation.
///
/// Content is a plain `String` for now. Multi-part / multimodal content is a
/// planned extension and will land as a breaking change pre-1.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// Who authored this message.
    pub role: Role,
    /// The message text.
    pub content: String,
}

impl Message {
    /// Build a [`Role::System`] message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }
    /// Build a [`Role::User`] message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }
    /// Build a [`Role::Assistant`] message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
    /// Build a [`Role::Tool`] message.
    pub fn tool(content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
        }
    }
}

/// The input to [`Agent::invoke`](crate::Agent::invoke).
///
/// Exists so callers can pass either a bare string (the common case) or a full
/// message history. The `From` impls make `agent.invoke("hi")` and
/// `agent.invoke(vec![msg1, msg2])` both work.
#[derive(Debug, Clone)]
pub struct Prompt {
    /// The messages to send, in order.
    pub messages: Vec<Message>,
}

impl From<&str> for Prompt {
    fn from(s: &str) -> Self {
        Self {
            messages: vec![Message::user(s)],
        }
    }
}
impl From<String> for Prompt {
    fn from(s: String) -> Self {
        Self {
            messages: vec![Message::user(s)],
        }
    }
}
impl From<Message> for Prompt {
    fn from(m: Message) -> Self {
        Self { messages: vec![m] }
    }
}
impl From<Vec<Message>> for Prompt {
    fn from(messages: Vec<Message>) -> Self {
        Self { messages }
    }
}
