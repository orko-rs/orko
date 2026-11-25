//! Encoding and decoding for the OpenAI Chat Completions API. Nothing here
//! does IO or needs a runtime.
//!
//! [`build_request`] turns an [`orko_core::CompletionRequest`] into the JSON
//! body, and [`parse_stream`] turns the SSE response bytes into
//! [`CompletionChunk`]s. A custom transport can use both as they are.

use crate::Quirks;
use eventsource_stream::Eventsource;
use futures::{future, Stream, StreamExt};
use orko_core::{
    CompletionChunk, CompletionRequest, ContentPart, ContentView, Error, FinishReason, MediaSource,
    Message, Result, Role, ToolCallDelta, Usage,
};
use serde_json::{json, Map, Value};

/// Builds the `POST /chat/completions` JSON body for `request`.
///
/// `default_model` is used when the request does not name a model.
///
/// # Errors
///
/// [`Error::Config`] if no model is set, or if a content part can't be sent
/// to Chat Completions (a file given by URL, or a [`ContentPart`] variant
/// added after this codec was written).
pub fn build_request(
    request: &CompletionRequest,
    default_model: &str,
    quirks: &Quirks,
) -> Result<Value> {
    let opts = &request.options;
    let model = opts.model.as_deref().unwrap_or(default_model);
    if model.is_empty() {
        return Err(Error::Config("no model specified".into()));
    }

    let messages = request
        .messages
        .iter()
        .map(message)
        .collect::<Result<Vec<_>>>()?;

    let mut body = Map::new();
    body.insert("model".into(), model.into());
    body.insert("messages".into(), messages.into());
    body.insert("stream".into(), true.into());
    if quirks.stream_usage {
        body.insert("stream_options".into(), json!({ "include_usage": true }));
    }
    if let Some(t) = opts.temperature {
        body.insert("temperature".into(), t.into());
    }
    if let Some(m) = opts.max_tokens {
        body.insert(quirks.max_tokens_field.into(), m.into());
    }
    if !opts.tools.is_empty() {
        let tools: Vec<Value> = opts
            .tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters,
                    }
                })
            })
            .collect();
        body.insert("tools".into(), tools.into());
    }
    Ok(Value::Object(body))
}

fn message(m: &Message) -> Result<Value> {
    let role = match m.role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        // TODO: core's agent loop writes tool results as plain text without a
        // `tool_call_id`, and the API needs one for `role: "tool"`. Send them
        // as user messages until core has structured tool-call messages.
        Role::Tool => "user",
    };
    let content = match m.content.pull() {
        ContentView::Text(t) => Value::String(t.to_owned()),
        ContentView::Parts(parts) => parts.iter().map(part).collect::<Result<Vec<_>>>()?.into(),
    };
    Ok(json!({ "role": role, "content": content }))
}

fn part(p: &ContentPart) -> Result<Value> {
    Ok(match p {
        ContentPart::Text { text } => json!({ "type": "text", "text": text }),
        ContentPart::Image { source } => {
            json!({ "type": "image_url", "image_url": { "url": media_url(source)? } })
        }
        ContentPart::File {
            source: MediaSource::Base64 { mime_type, data },
            name,
        } => {
            let mut file = Map::new();
            file.insert(
                "file_data".into(),
                format!("data:{mime_type};base64,{data}").into(),
            );
            if let Some(n) = name {
                file.insert("filename".into(), n.as_str().into());
            }
            json!({ "type": "file", "file": file })
        }
        ContentPart::File { .. } => {
            return Err(Error::Config(
                "chat completions accepts files only as inline base64 data".into(),
            ))
        }
        _ => return Err(Error::Config("unsupported content part".into())),
    })
}

fn media_url(source: &MediaSource) -> Result<String> {
    match source {
        MediaSource::Url { url } => Ok(url.clone()),
        MediaSource::Base64 { mime_type, data } => Ok(format!("data:{mime_type};base64,{data}")),
        _ => Err(Error::Config("unsupported media source".into())),
    }
}

/// Parses an SSE byte stream from `/chat/completions` into completion chunks.
///
/// The stream ends at `data: [DONE]` or when the bytes run out. An
/// `{"error": ...}` event comes through as an [`Error::Provider`] item.
pub fn parse_stream<S, B, E>(bytes: S) -> impl Stream<Item = Result<CompletionChunk>> + Send
where
    S: Stream<Item = std::result::Result<B, E>> + Send,
    B: AsRef<[u8]>,
    E: std::fmt::Display + Send,
{
    bytes
        .eventsource()
        .take_while(|ev| future::ready(!matches!(ev, Ok(e) if e.data.trim() == "[DONE]")))
        .filter_map(|ev| {
            future::ready(match ev {
                Ok(e) => parse_event(&e.data).transpose(),
                Err(e) => Some(Err(Error::Provider(format!("stream error: {e}")))),
            })
        })
}

/// Parses one SSE `data:` payload. Returns `Ok(None)` if the event has nothing in it.
pub fn parse_event(data: &str) -> Result<Option<CompletionChunk>> {
    if data.trim().is_empty() {
        return Ok(None);
    }
    let v: Value = serde_json::from_str(data)?;
    if let Some(err) = v.get("error") {
        let msg = err
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        return Err(Error::Provider(msg.to_owned()));
    }

    let mut chunk = CompletionChunk::default();
    // INFO: reads only the first choice, since we never ask for `n > 1`.
    if let Some(choice) = v.pointer("/choices/0") {
        if let Some(delta) = choice.get("delta") {
            let s = |k: &str| delta.get(k).and_then(Value::as_str).map(str::to_owned);
            chunk.content = s("content").unwrap_or_default();
            // DeepSeek calls it `reasoning_content`, OpenRouter calls it `reasoning`.
            chunk.reasoning = s("reasoning_content").or_else(|| s("reasoning"));
            chunk.refusal = s("refusal");
            if let Some(calls) = delta.get("tool_calls").and_then(Value::as_array) {
                chunk.tool_calls = calls
                    .iter()
                    .enumerate()
                    .map(|(position, call)| tool_call(call, position as u32))
                    .collect();
            }
        }
        chunk.finish_reason = match choice.get("finish_reason").and_then(Value::as_str) {
            Some("stop") => Some(FinishReason::Stop),
            Some("length") => Some(FinishReason::Length),
            Some("tool_calls" | "function_call") => Some(FinishReason::ToolCalls),
            Some("content_filter") => Some(FinishReason::ContentFilter),
            _ => None,
        };
    }
    if let Some(u) = v.get("usage").filter(|u| !u.is_null()) {
        let n = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0) as u32;
        chunk.usage = Some(Usage {
            prompt_tokens: n("prompt_tokens"),
            completion_tokens: n("completion_tokens"),
        });
    }

    let empty = chunk.content.is_empty()
        && chunk.reasoning.is_none()
        && chunk.refusal.is_none()
        && chunk.tool_calls.is_empty()
        && chunk.finish_reason.is_none()
        && chunk.usage.is_none();
    Ok((!empty).then_some(chunk))
}

fn tool_call(c: &Value, position: u32) -> ToolCallDelta {
    let f = c.get("function");
    let s = |v: Option<&Value>| v.and_then(Value::as_str).map(str::to_owned);
    ToolCallDelta {
        index: c
            .get("index")
            .and_then(Value::as_u64)
            .map_or(position, |index| index as u32),
        id: s(c.get("id")),
        name: s(f.and_then(|f| f.get("name"))),
        arguments: s(f.and_then(|f| f.get("arguments"))).unwrap_or_default(),
    }
}
