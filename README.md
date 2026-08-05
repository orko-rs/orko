# Orko

> **Agent orchestration toolkit for Rust** — efficient, durable, streaming-first agent orchestration

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.82%2B-orange.svg)](Cargo.toml)
[![Status](https://img.shields.io/badge/status-early%20development-yellow.svg)](#development-status)

```rust
use orko_core::{create_agent, Provider, Result};

async fn demo(p: impl Provider) -> Result<()> {
    let agent = create_agent(p)
        .with_system_prompt("You are an agent orchestrated with Orko.")
        .build();
    let reply = agent.invoke("Hello!").await?;
    println!("{reply}");
    Ok(())
}
```

## Development status

| Crate                                     | Purpose                                                                            |     Status     | Notes                                                                      |
| ----------------------------------------- | ---------------------------------------------------------------------------------- | :------------: | -------------------------------------------------------------------------- |
| [`orko-core`](crates/orko-core)           | Core traits & types — `Provider`, `Agent`, `Tool`, `ModelRouter`, streaming chunks | **reforming**  | API-frozen · 4 unit tests + 1 doctest passing · `clippy -D warnings` clean |
| [`orko-macros`](crates/orko-macros)       | Proc macros (ergonomic tool definitions)                                           | 🩻 Scaffolded  | Empty stub — next in build order                                           |
| [`orko-providers`](crates/orko-providers) | Provider implementations (OpenAI-compatible HTTP + SSE, local inference)           | 🩻 Scaffolded  | Empty stub                                                                 |
| [`orko-graph`](crates/orko-graph)         | Graph-based multi-agent workflows                                                  | 🩻 Scaffolded  | Empty stub                                                                 |
| [`orko-runtime`](crates/orko-runtime)     | Tokio runtime integration & task spawning                                          | 🩻 Scaffolded  | Empty stub — the _only_ crate allowed to touch tokio                       |
| [`orko-mcp`](crates/orko-mcp)             | Model Context Protocol integration                                                 | 🩻 Scaffolded  | Empty stub                                                                 |
| `orko` (facade)                           | Single-dependency entry point re-exporting the workspace                           | ⬜ Not started | Created last, once the pieces exist                                        |

<!--### What works today (`orko-core`)

| Area      | Delivered                                                                                                                       |
| --------- | ------------------------------------------------------------------------------------------------------------------------------- |
| Messages  | `Role` / `Message` / `Prompt` with ergonomic `From` conversions; wire format test-locked to `{"role":"user","content":"hello"}` |
| Providers | Generic-first `Provider` trait + object-safe `DynProvider` / `BoxProvider` twin, so both static and dynamic dispatch work       |
| Streaming | `CompletionStream` of `CompletionChunk` deltas — content, reasoning, refusals, tool-call fragments, finish reason, usage        |
| Tools     | Object-safe `Tool` trait + `ToolSpec` (JSON-Schema parameters)                                                                  |
| Agents    | `create_agent(provider)` builder — system prompt, model, tools; `invoke` / `invoke_stream`                                      |
| Routing   | `ModelRouter` trait + `StaticRouter`                                                                                            |

Deliberately deferred (tracked, not forgotten): `tool_choice` option,
tool-call/tool-result message variants, and the agent tool-execution loop.-->

## Architecture

```mermaid
graph TD
    facade["orko"]
    macros[orko-macros]
    providers[orko-providers]
    runtime[orko-runtime]
    graphcrate[orko-graph]
    mcp[orko-mcp]
    core["orko-core"]

    facade --> macros
    facade --> providers
    facade --> runtime
    facade --> graphcrate
    facade --> mcp

    macros --> core
    providers --> core
    runtime --> core
    runtime -. sole exception .-> graphcrate
    graphcrate --> core
    mcp --> core

    style core fill:#e8f5e9,stroke:#2e7d32,color:#000
    style facade fill:#fff8e1,stroke:#f9a825,color:#000,stroke-dasharray: 5 5
```

Arrows mean "depends on": everything points down at `orko-core`, the facade (orko)
points at everything, and the one dotted edge is the single allowed extra
dependency (`orko-runtime` → `orko-graph`).

The rules that keep it honest:

1. **Dependencies flow down** — every crate depends only on `orko-core`
   (sole exception: `orko-runtime` may also depend on `orko-graph`).
2. **No tokio in public APIs** — public traits use `std::future::Future` and
   `futures::Stream` only; tokio lives exclusively inside `orko-runtime`.
3. **`orko-core` stays lean** — exactly four dependencies (`serde`,
   `serde_json`, `thiserror`, `futures`), no HTTP, no runtime, `#![forbid(unsafe_code)]`.

## Getting started

```sh
git clone https://github.com/orko-rs/orko
cd orko
cargo test -p orko-core
cargo clippy -p orko-core --all-targets -- -D warnings
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
