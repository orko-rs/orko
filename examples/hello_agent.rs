//! One example for every preset.
//!
//! ```sh
//! cargo run --example hello_agent                                      # offline stub
//! cargo run --example hello_agent -- openai:gpt-4o-mini                # OPENAI_API_KEY
//! cargo run --example hello_agent -- groq:llama-3.3-70b-versatile      # GROQ_API_KEY
//! cargo run --example hello_agent -- ollama:llama3.2                   # local, no key
//! ```
//!
//! The argument is a `provider:model` spec for `orko::providers::from_str`. With no
//! argument it runs offline through `Canned`, a fake transport that returns a
//! fixed SSE response.

use orko::providers::{from_str, HttpResponse, OpenAiCompatible, Transport};
use orko::{boxed, create_agent, Result};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    let provider = match std::env::args().nth(1) {
        Some(spec) => from_str(&spec)?,
        None => {
            eprintln!("no `provider:model` given; using the offline stub transport");
            boxed(OpenAiCompatible::new("http://offline", "stub").with_transport(Arc::new(Canned)))
        }
    };
    let agent = create_agent(provider)
        .with_system_prompt("You are a helpful assistant. Be brief.")
        .build()?;
    println!("{}", agent.invoke("What is Rust, in one sentence?").await?);
    Ok(())
}

/// Replies "Hello from the offline stub!" to every request.
struct Canned;

impl Transport for Canned {
    fn post<'a>(
        &'a self,
        _url: &'a str,
        _headers: &'a [(String, String)],
        _body: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse>> + Send + 'a>> {
        let sse = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hello from the offline stub!\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n",
        );
        Box::pin(async move {
            Ok(HttpResponse {
                status: 200,
                body: Box::pin(futures::stream::iter([Ok(sse.as_bytes().to_vec())])),
            })
        })
    }
}
