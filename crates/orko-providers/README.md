# orko-providers

Providers for [orko](https://github.com/orko-rs/orko) that speak the OpenAI Chat Completions API.

```rust
let agent = orko_core::create_agent(orko_providers::openai("gpt-4o-mini")).build()?;
println!("{}", agent.invoke("Hello!").await?);
```

Each preset is a function that takes a model id. Hosted ones read their key from an environment variable; local ones send no key unless one is set.

| Function | Base URL | Key |
|---|---|---|
| `openai` | `https://api.openai.com/v1` | `OPENAI_API_KEY` |
| `deepseek` | `https://api.deepseek.com/v1` | `DEEPSEEK_API_KEY` |
| `gemini` | `https://generativelanguage.googleapis.com/v1beta/openai` | `GEMINI_API_KEY` |
| `groq` | `https://api.groq.com/openai/v1` | `GROQ_API_KEY` |
| `together` | `https://api.together.xyz/v1` | `TOGETHER_API_KEY` |
| `openrouter` | `https://openrouter.ai/api/v1` | `OPENROUTER_API_KEY` |
| `moonshot` | `https://api.moonshot.ai/v1` | `MOONSHOT_API_KEY` |
| `qwen` | `https://dashscope-intl.aliyuncs.com/compatible-mode/v1` | `DASHSCOPE_API_KEY` |
| `mistral` | `https://api.mistral.ai/v1` | `MISTRAL_API_KEY` |
| `xai` | `https://api.x.ai/v1` | `XAI_API_KEY` |
| `perplexity` | `https://api.perplexity.ai` | `PERPLEXITY_API_KEY` |
| `nvidia` | `https://integrate.api.nvidia.com/v1` | `NVIDIA_API_KEY` |
| `zai` | `https://api.z.ai/api/paas/v4` | `ZAI_API_KEY` |
| `huggingface` | `https://router.huggingface.co/v1` | `HF_TOKEN` |
| `ollama` | `http://localhost:11434/v1` | none |
| `vllm` | `http://localhost:8000/v1` | `VLLM_API_KEY` (optional) |
| `llamacpp` | `http://localhost:8080/v1` | `LLAMACPP_API_KEY` (optional) |

For any other endpoint, such as a server on another host, use `OpenAiCompatible::new(base_url, model)`.

You can also build a provider from a string: `orko_providers::from_str("deepseek:deepseek-chat")?` returns a `BoxProvider`. The part before the first `:` is the function name, so `"ollama:llama3.2:3b"` works too.

## How it fits together

The crate has three parts. `orko_providers::codec` builds the request JSON and parses the SSE stream into `CompletionChunk`s, without doing any IO. `orko_providers::Transport` is a small object-safe trait that POSTs bytes with headers and returns the response body as a stream. `OpenAiCompatible` uses both and implements `orko_core::Provider`. A preset such as `openai` is an `OpenAiCompatible` with the right base URL, key and `Quirks`.

## Features

| Feature | Default | What it adds |
|---|---|---|
| `transport-tokio` | yes | `TokioTransport`, which runs hyper 1.x HTTP/1.1 on Tokio through hyper-util and hyper-rustls, with the platform's root certificates. It has to run inside a Tokio runtime and spawns no tasks. Requests time out after 60 s by default; change it with `with_timeout`. |

With `--no-default-features` you get the codec and `OpenAiCompatible`, and you supply the transport.

## Using your own HTTP client

To use another runtime, or to add a proxy, custom TLS or connection pooling, implement `Transport` and pass it to `OpenAiCompatible::with_transport`. Request building and parsing work the same way.

If you need more control than that, implement `orko_core::Provider` yourself with any HTTP client. The rest of orko doesn't depend on this crate.

## Known limits

- `TokioTransport` opens a new connection for every request.
- The timeout covers connecting, TLS and waiting for the response head. A stream that stops partway through can still hang.
- Tool results go out as `user` messages, because `orko-core` doesn't carry a `tool_call_id` yet.
- Files have to be inline base64, since Chat Completions doesn't accept file URLs.
