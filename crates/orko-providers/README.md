# orko-providers

Providers for [orko](https://github.com/orko-rs/orko) that speak the OpenAI Chat Completions API.

```rust
let agent = orko_core::create_agent(orko_providers::openai("gpt-4o-mini")).build()?;
println!("{}", agent.invoke("Hello!").await?);
```

There are two presets. `openai(model)` reads its key from `OPENAI_API_KEY`. `deepseek(model)` reads `DEEPSEEK_API_KEY` and works with `deepseek-chat` and `deepseek-reasoner`.

You can also build a provider from a string: `orko_providers::from_str("deepseek:deepseek-chat")?` returns a `BoxProvider`.

## How it fits together

The crate has three parts. `orko_providers::codec` builds the request JSON and parses the SSE stream into `CompletionChunk`s, without doing any IO. `orko_providers::Transport` is a small object-safe trait that POSTs bytes with headers and returns the response body as a stream. `OpenAiCompatible` uses both and implements `orko_core::Provider`. A preset such as `openai` is an `OpenAiCompatible` with the right base URL, key and `Quirks`.

## Features

| Feature | Default | What it adds |
|---|---|---|
| `transport-hyper` | yes | `HyperTransport`, which runs hyper 1.x HTTP/1.1 over `async-net` TCP, with rustls and the platform's root certificates. It works under Tokio, smol or `futures::executor::block_on` and spawns no tasks. Requests time out after 60 s by default; change it with `with_timeout`. |

With `--no-default-features` you get the codec and `OpenAiCompatible`, and you supply the transport.

## Using your own HTTP client

To add a proxy, custom TLS or connection pooling, or to use a different runtime's networking, implement `Transport` and pass it to `OpenAiCompatible::with_transport`. Request building and parsing work the same way.

If you need more control than that, implement `orko_core::Provider` yourself with any HTTP client. The rest of orko doesn't depend on this crate.

## Known limits

- `HyperTransport` opens a new connection for every request.
- The timeout covers connecting, TLS and waiting for the response head. A stream that stops partway through can still hang.
- Tool results go out as `user` messages, because `orko-core` doesn't carry a `tool_call_id` yet.
- Files have to be inline base64, since Chat Completions doesn't accept file URLs.
