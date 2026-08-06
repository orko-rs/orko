// TODO: runtime design — Tokio contained here, agnosticism preserved:
// - One internal `fn spawn(...)` (+ `sleep`) shim; Tokio types never appear in
//   a public signature.
// - Runtime backends behind cargo features, sqlx-style: `rt-tokio` (default),
//   `rt-smol`, … — each backend implements only the shim.
// - Bring-your-own-executor escape hatch: an object-safe, std-only
//   `trait Executor { fn spawn(&self, fut: Pin<Box<dyn Future<Output = ()> + Send>>); }`
//   accepted as `Arc<dyn Executor>` (or adopt `futures::task::Spawn`) for
//   runtimes we don't feature-gate.
// - orko-core stays service-free: concurrency there uses futures combinators
//   (`join_all` / `FuturesUnordered`), never spawn — see the dispatch TODO in
//   orko-core's agent.rs.
