//! [`OpenAiCompatible`], which sends codec output over a [`Transport`].

use crate::codec;
use crate::transport::Transport;
use futures::StreamExt;
use orko_core::{CompletionRequest, CompletionStream, Error, Provider, Result};
use std::future::Future;
use std::sync::Arc;

/// Ways an OpenAI-compatible API differs from OpenAI's own.
///
/// Start from [`Quirks::default`] and change the fields you need.
///
/// Response fields that only differ by name don't need a quirk. The codec
/// reads both `reasoning_content` (DeepSeek) and `reasoning` (OpenRouter)
/// into [`orko_core::CompletionChunk::reasoning`], and it ignores fields it
/// doesn't know.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Quirks {
    /// Send `stream_options: {"include_usage": true}`. Some servers reject it.
    pub stream_usage: bool,
    /// Name of the token limit field: `"max_tokens"` or `"max_completion_tokens"`.
    pub max_tokens_field: &'static str,
}

impl Default for Quirks {
    fn default() -> Self {
        Self {
            stream_usage: true,
            max_tokens_field: "max_tokens",
        }
    }
}

/// An OpenAI Chat Completions-compatible provider.
///
/// OpenAI, DeepSeek, Groq, Ollama and the other compatible APIs all use this
/// type with their own base URL, key and [`Quirks`].
#[derive(Clone)]
pub struct OpenAiCompatible {
    base_url: String,
    model: String,
    api_key: Option<String>,
    quirks: Quirks,
    transport: Option<Arc<dyn Transport>>,
}

impl std::fmt::Debug for OpenAiCompatible {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAiCompatible")
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("quirks", &self.quirks)
            .finish_non_exhaustive()
    }
}

impl OpenAiCompatible {
    /// Creates a provider for `base_url` (e.g. `https://api.openai.com/v1`)
    /// with `model` as the default model, no API key and default quirks.
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            api_key: None,
            quirks: Quirks::default(),
            transport: None,
        }
    }

    /// Sends `Authorization: Bearer <key>`.
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Reads the bearer key from the environment variable `var`, if it is set.
    pub fn with_api_key_env(mut self, var: &str) -> Self {
        self.api_key = std::env::var(var).ok().filter(|k| !k.is_empty());
        self
    }

    /// Replaces the quirks.
    pub fn with_quirks(mut self, quirks: Quirks) -> Self {
        self.quirks = quirks;
        self
    }

    /// Sends requests through `transport` instead of the default one.
    pub fn with_transport(mut self, transport: Arc<dyn Transport>) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Returns the default model id.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Returns the API base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn transport(&self) -> Result<Arc<dyn Transport>> {
        if let Some(t) = &self.transport {
            return Ok(t.clone());
        }
        default_transport()
    }

    async fn run(&self, request: CompletionRequest) -> Result<CompletionStream> {
        let body = codec::build_request(&request, &self.model, &self.quirks)?;
        let body = serde_json::to_vec(&body)?;
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let mut headers = vec![
            ("content-type".to_owned(), "application/json".to_owned()),
            ("accept".to_owned(), "text/event-stream".to_owned()),
        ];
        if let Some(key) = &self.api_key {
            headers.push(("authorization".to_owned(), format!("Bearer {key}")));
        }

        let transport = self.transport()?;
        let resp = transport.post(&url, &headers, body).await?;
        if !(200..300).contains(&resp.status) {
            // ponytail: error bodies are short JSON, so 8 KiB is plenty.
            let mut text = Vec::new();
            let mut body = resp.body;
            while let Some(part) = body.next().await {
                text.extend_from_slice(&part?);
                if text.len() > 8192 {
                    break;
                }
            }
            return Err(Error::Provider(format!(
                "HTTP {}: {}",
                resp.status,
                String::from_utf8_lossy(&text).trim()
            )));
        }
        Ok(Box::pin(codec::parse_stream(resp.body)))
    }
}

impl Provider for OpenAiCompatible {
    fn complete(
        &self,
        request: CompletionRequest,
    ) -> impl Future<Output = Result<CompletionStream>> + Send {
        self.run(request)
    }
}

#[cfg(feature = "transport-hyper")]
fn default_transport() -> Result<Arc<dyn Transport>> {
    use std::sync::OnceLock;
    static DEFAULT: OnceLock<Arc<dyn Transport>> = OnceLock::new();
    if let Some(t) = DEFAULT.get() {
        return Ok(t.clone());
    }
    let t: Arc<dyn Transport> = Arc::new(crate::transport::HyperTransport::new()?);
    Ok(DEFAULT.get_or_init(|| t).clone())
}

#[cfg(not(feature = "transport-hyper"))]
fn default_transport() -> Result<Arc<dyn Transport>> {
    Err(Error::Config(
        "no transport: enable the `transport-hyper` feature or call `with_transport`".into(),
    ))
}
