//! `cargo run --example hello_deepseek`
//!
//! Calls DeepSeek if `DEEPSEEK_API_KEY` is set. Otherwise it runs offline with
//! `common::Canned`, a fake transport that returns a fixed SSE response.

mod common;

use common::Canned;
use orko::providers::deepseek;
use orko::{create_agent, Result};
use std::sync::Arc;

// Any executor works. `block_on` is the simplest.
fn main() -> Result<()> {
    futures::executor::block_on(run())
}

async fn run() -> Result<()> {
    let mut provider = deepseek("deepseek-chat");
    if std::env::var_os("DEEPSEEK_API_KEY").is_none_or(|v| v.is_empty()) {
        eprintln!("DEEPSEEK_API_KEY not set; using the offline stub transport");
        provider = provider.with_transport(Arc::new(Canned("the offline DeepSeek stub!")));
    }
    let agent = create_agent(provider)
        .with_system_prompt("You are a helpful assistant. Be brief.")
        .build()?;
    println!("{}", agent.invoke("What is Rust, in one sentence?").await?);
    Ok(())
}
