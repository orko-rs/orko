// TODO: engine design — codec/transport split, hyper as the default transport
// (decision 2026-08-07, replacing the earlier reqwest plan):
// - The OpenAI-compat *codec* (request JSON build, SSE parse into
//   `CompletionChunk` via eventsource-stream) is pure computation — no IO, no
//   runtime; keep it that way so it stays agnostic by construction.
// - The *transport* is one tiny internal object-safe trait, std vocabulary
//   only: POST bytes + headers -> streaming byte response.
// - Default transport: hyper 1.x + hyper-util + hyper-rustls behind a
//   `transport-tokio` feature (default on). hyper 1.x is runtime-agnostic via
//   its `hyper::rt` traits; a smol/other transport is a ~100-line addition,
//   not a second engine. We drive the connection with `select!`, no spawn.
// - Users with exotic needs (proxies, custom TLS, own pooling) inject their
//   own transport; the codec never changes.
// - The deeper any-runtime escape hatch remains `orko_core::Provider` itself:
//   implement it with any HTTP client and the rest of orko works untouched.
//   Document both hatches in the crate README.
// TODO: add a crate README in every crate.
