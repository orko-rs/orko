# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/). Before 1.0, a minor version bump
(0.1 to 0.2) can contain breaking changes.

## [0.1.0] - 2026-11-21

First release. It publishes `orko-core`, `orko-macros`, `orko-providers` and
the `orko` facade.

### orko-core

- `Provider` trait with streaming `complete()`, plus `DynProvider` and
  `BoxProvider` for type erasure. `Arc<P>` is a `Provider` too.
- Request and stream types: `CompletionRequest`, `CompletionOptions`,
  `CompletionChunk`, `ToolCallDelta`, `FinishReason`, `Usage`.
- `Message` and `Content`: plain text or a list of parts (text, image, file),
  with media given as a URL or as base64 with a MIME type.
- Object-safe `Tool` trait, with `call_blocking` for use outside an async
  runtime.
- `Agent` and its builder. `invoke` runs the full tool loop up to
  `with_max_tool_turns` (default 8) and sends tool errors back to the model so
  it can correct itself. `invoke_stream` streams a single turn.
- `build()` rejects duplicate tool names with `Error::Config`.

### orko-macros

- `#[tool]` turns a plain or `async` function into a constructor that returns
  `Arc<dyn Tool>`. The description comes from `desc = "..."` or the doc
  comment, and the JSON schema comes from the parameter types.

### orko-providers

- `OpenAiCompatible`: one engine for the OpenAI Chat Completions API, with
  `Quirks` for per-provider differences.
- Presets: `openai`, `deepseek`, `gemini`, `groq`, `together`, `openrouter`,
  `moonshot`, `qwen`, `mistral`, `xai`, `perplexity`, `nvidia`, `zai`,
  `huggingface`, `ollama`, `vllm`, `llamacpp`. Hosted presets read their key
  from the usual environment variable.
- `from_str("provider:model")` builds any preset as a `BoxProvider`.
- `Transport` trait for bringing your own HTTP client, and the built-in
  `TokioTransport` (hyper 1.x, rustls, platform root certificates) behind the
  default `transport-tokio` feature. Requests time out after 60 seconds by
  default.

### orko

- Facade crate. Re-exports `orko-core` at the root, `orko::providers`
  (default feature `providers`) and `#[tool]` (feature `macros`). Feature
  `full` turns on both.
- Examples `hello_openai` and `hello_deepseek`. Without an API key they
  replay a canned response.

### Known limitations

- `TokioTransport` opens a new connection for every request and only speaks
  HTTP/1.1.
- The request timeout doesn't cover the time between body chunks, so a
  stream that stalls partway through can hang.
- There are no retries. A 429 or 5xx is returned as `Error::Provider`.
- Tool results go to the model as `user` messages, because `Message` has no
  `tool_call_id` yet.
- Files must be inline base64.
- `#[tool]` expands to `::orko_core` paths, so a crate that uses it through
  the facade also needs `orko-core` as a direct dependency.
- Anthropic, Azure OpenAI, AWS Bedrock and Candle are not supported yet.
- `orko-graph`, `orko-mcp` and `orko-runtime` are placeholders and are not
  published. Their facade features will return when they have code.

[0.1.0]: https://github.com/orko-rs/orko/releases/tag/v0.1.0
