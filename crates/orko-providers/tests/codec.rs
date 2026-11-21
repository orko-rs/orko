use futures::executor::block_on;
use futures::{stream, StreamExt};
use orko_core::*;
use orko_providers::codec::{build_request, parse_stream};
use orko_providers::{
    deepseek, from_str, openai, HttpResponse, OpenAiCompatible, Quirks, Transport,
};
use serde_json::json;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

fn quirks() -> Quirks {
    Quirks::default()
}

#[test]
fn text_request_keeps_plain_string_content() {
    let req = CompletionRequest::new(vec![Message::system("be brief"), Message::user("hi")]);
    let body = build_request(&req, "gpt-4o-mini", &quirks()).unwrap();
    assert_eq!(
        body,
        json!({
            "model": "gpt-4o-mini",
            "messages": [
                {"role": "system", "content": "be brief"},
                {"role": "user", "content": "hi"}
            ],
            "stream": true,
            "stream_options": {"include_usage": true}
        })
    );
}

#[test]
fn options_tools_and_quirks_shape_the_body() {
    let mut q = quirks();
    q.stream_usage = false;
    q.max_tokens_field = "max_completion_tokens";
    let req = CompletionRequest::new(vec![Message::user("add")]).with_options(CompletionOptions {
        model: Some("override".into()),
        temperature: Some(0.5),
        max_tokens: Some(64),
        tools: vec![ToolSpec {
            name: "add".into(),
            description: "Adds.".into(),
            parameters: json!({"type": "object"}),
        }],
    });
    let body = build_request(&req, "default", &q).unwrap();
    assert_eq!(
        body,
        json!({
            "model": "override",
            "messages": [{"role": "user", "content": "add"}],
            "stream": true,
            "temperature": 0.5,
            "max_completion_tokens": 64,
            "tools": [{
                "type": "function",
                "function": {"name": "add", "description": "Adds.", "parameters": {"type": "object"}}
            }]
        })
    );
}

#[test]
fn multipart_content_uses_array_form_and_data_urls() {
    let req = CompletionRequest::new(vec![Message::user(vec![
        ContentPart::text("what is this?"),
        ContentPart::image_url("https://example.com/cat.png"),
        ContentPart::image(MediaSource::base64("image/png", "AAAA")),
        ContentPart::File {
            source: MediaSource::base64("application/pdf", "aGVsbG8="),
            name: Some("hello.pdf".into()),
        },
    ])]);
    let body = build_request(&req, "m", &quirks()).unwrap();
    assert_eq!(
        body["messages"][0]["content"],
        json!([
            {"type": "text", "text": "what is this?"},
            {"type": "image_url", "image_url": {"url": "https://example.com/cat.png"}},
            {"type": "image_url", "image_url": {"url": "data:image/png;base64,AAAA"}},
            {"type": "file", "file": {"file_data": "data:application/pdf;base64,aGVsbG8=", "filename": "hello.pdf"}}
        ])
    );
}

#[test]
fn tool_role_is_sent_as_user_and_file_urls_are_rejected() {
    let req = CompletionRequest::new(vec![Message::tool("[add:call_1] 3")]);
    let body = build_request(&req, "m", &quirks()).unwrap();
    assert_eq!(body["messages"][0]["role"], "user");

    let req = CompletionRequest::new(vec![Message::user(vec![ContentPart::file(
        MediaSource::url("https://example.com/a.pdf"),
    )])]);
    assert!(matches!(
        build_request(&req, "m", &quirks()),
        Err(Error::Config(_))
    ));
    let req = CompletionRequest::new(vec![Message::user("hi")]);
    assert!(matches!(
        build_request(&req, "", &quirks()),
        Err(Error::Config(_))
    ));
}

/// Feeds `sse` in 7-byte pieces so events get split across chunks.
fn parse(sse: &str) -> Vec<Result<CompletionChunk>> {
    let pieces: Vec<std::result::Result<Vec<u8>, Infallible>> =
        sse.as_bytes().chunks(7).map(|c| Ok(c.to_vec())).collect();
    block_on(parse_stream(stream::iter(pieces)).collect())
}

#[test]
fn text_stream_with_usage_and_done() {
    let sse = "\
data: {\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"\"}}]}\n\n\
data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\n\
: keep-alive comment\n\n\
data: {\"choices\":[{\"delta\":{\"content\":\"lo\"},\"finish_reason\":\"stop\"}]}\n\n\
data: {\"choices\":[],\"usage\":{\"prompt_tokens\":9,\"completion_tokens\":2}}\n\n\
data: [DONE]\n\n\
data: {\"choices\":[{\"delta\":{\"content\":\"after done\"}}]}\n\n";
    let chunks: Vec<_> = parse(sse).into_iter().map(Result::unwrap).collect();
    let text: String = chunks.iter().map(|c| c.content.as_str()).collect();
    assert_eq!(text, "Hello");
    assert_eq!(
        chunks.len(),
        3,
        "role-only delta skipped, nothing after [DONE]"
    );
    assert_eq!(chunks[1].finish_reason, Some(FinishReason::Stop));
    assert_eq!(
        chunks[2].usage,
        Some(Usage {
            prompt_tokens: 9,
            completion_tokens: 2
        })
    );
}

#[test]
fn multi_tool_call_deltas_reassemble_by_index() {
    let sse = concat!(
        r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_a","type":"function","function":{"name":"add","arguments":""}}]}}]}"#,
        "\n\n",
        r#"data: {"choices":[{"delta":{"tool_calls":[{"index":1,"id":"call_b","type":"function","function":{"name":"mul","arguments":"{\"a\":"}}]}}]}"#,
        "\n\n",
        r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"a\":1,"}}]}}]}"#,
        "\n\n",
        r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\"b\":2}"}},{"index":1,"function":{"arguments":"3}"}}]}}]}"#,
        "\n\n",
        r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
        "\n\n",
        "data: [DONE]\n\n",
    );
    let chunks: Vec<_> = parse(sse).into_iter().map(Result::unwrap).collect();

    let mut calls: Vec<(Option<String>, Option<String>, String)> = Vec::new();
    for d in chunks.iter().flat_map(|c| &c.tool_calls) {
        let i = d.index as usize;
        if calls.len() <= i {
            calls.resize(i + 1, Default::default());
        }
        calls[i].0 = calls[i].0.take().or(d.id.clone());
        calls[i].1 = calls[i].1.take().or(d.name.clone());
        calls[i].2.push_str(&d.arguments);
    }
    assert_eq!(
        calls,
        vec![
            (
                Some("call_a".into()),
                Some("add".into()),
                r#"{"a":1,"b":2}"#.into()
            ),
            (
                Some("call_b".into()),
                Some("mul".into()),
                r#"{"a":3}"#.into()
            ),
        ]
    );
    assert_eq!(
        chunks.last().unwrap().finish_reason,
        Some(FinishReason::ToolCalls)
    );
}

#[test]
fn reasoning_refusal_and_error_events() {
    let sse = concat!(
        r#"data: {"choices":[{"delta":{"reasoning_content":"think"}}]}"#,
        "\n\n",
        r#"data: {"choices":[{"delta":{"refusal":"no"},"finish_reason":"content_filter"}]}"#,
        "\n\n",
        r#"data: {"error":{"message":"overloaded"}}"#,
        "\n\n",
    );
    let out = parse(sse);
    assert_eq!(out[0].as_ref().unwrap().reasoning.as_deref(), Some("think"));
    let refusal = out[1].as_ref().unwrap();
    assert_eq!(refusal.refusal.as_deref(), Some("no"));
    assert_eq!(refusal.finish_reason, Some(FinishReason::ContentFilter));
    assert!(matches!(&out[2], Err(Error::Provider(m)) if m == "overloaded"));
}

/// (url, headers, JSON body) of the last request.
type Seen = (String, Vec<(String, String)>, serde_json::Value);

/// Records each request and returns the same response every time.
struct Stub {
    status: u16,
    body: &'static str,
    seen: Mutex<Option<Seen>>,
}

impl Transport for Stub {
    fn post<'a>(
        &'a self,
        url: &'a str,
        headers: &'a [(String, String)],
        body: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse>> + Send + 'a>> {
        *self.seen.lock().unwrap() = Some((
            url.to_owned(),
            headers.to_vec(),
            serde_json::from_slice(&body).unwrap(),
        ));
        let resp = HttpResponse {
            status: self.status,
            body: Box::pin(stream::iter([Ok(self.body.as_bytes().to_vec())])),
        };
        Box::pin(async move { Ok(resp) })
    }
}

fn stub(status: u16, body: &'static str) -> Arc<Stub> {
    Arc::new(Stub {
        status,
        body,
        seen: Mutex::new(None),
    })
}

#[test]
fn engine_runs_agent_through_injected_transport() {
    let t = stub(
        200,
        "data: {\"choices\":[{\"delta\":{\"content\":\"hi there\"}}]}\n\ndata: [DONE]\n\n",
    );
    let p = OpenAiCompatible::new("https://example.test/v1/", "m")
        .with_api_key("sk-test")
        .with_transport(t.clone());
    let agent = create_agent(p).build().unwrap();
    assert_eq!(block_on(agent.invoke("hello")).unwrap(), "hi there");

    let (url, headers, body) = t.seen.lock().unwrap().take().unwrap();
    assert_eq!(url, "https://example.test/v1/chat/completions");
    assert!(headers.contains(&("authorization".into(), "Bearer sk-test".into())));
    assert_eq!(body["model"], "m");
}

#[test]
fn non_2xx_status_becomes_provider_error() {
    let t = stub(401, r#"{"error":{"message":"bad key"}}"#);
    let p = OpenAiCompatible::new("https://example.test/v1", "m").with_transport(t);
    let err = block_on(p.complete(CompletionRequest::new(vec![Message::user("x")])))
        .err()
        .unwrap();
    assert!(matches!(err, Error::Provider(m) if m.contains("401") && m.contains("bad key")));
}

#[test]
fn openai_preset_and_from_str() {
    let p = openai("gpt-4o-mini");
    assert_eq!(p.model(), "gpt-4o-mini");
    assert!(from_str("openai:gpt-4o-mini").is_ok());
    for bad in ["openai", "openai:", ":gpt", "nope:gpt"] {
        assert!(matches!(from_str(bad), Err(Error::Config(_))), "{bad}");
    }
}

#[test]
fn deepseek_preset_config_and_from_str() {
    std::env::set_var("DEEPSEEK_API_KEY", "sk-ds");
    let t = stub(200, "data: [DONE]\n\n");
    let p = deepseek("deepseek-reasoner").with_transport(t.clone());
    assert_eq!(p.base_url(), "https://api.deepseek.com/v1");
    assert_eq!(p.model(), "deepseek-reasoner");

    let req = CompletionRequest::new(vec![Message::user("x")]).with_options(CompletionOptions {
        max_tokens: Some(32),
        ..Default::default()
    });
    let _ = block_on(p.complete(req)).unwrap();
    let (url, headers, body) = t.seen.lock().unwrap().take().unwrap();
    assert_eq!(url, "https://api.deepseek.com/v1/chat/completions");
    assert!(headers.contains(&("authorization".into(), "Bearer sk-ds".into())));
    assert_eq!(body["max_tokens"], 32);
    assert_eq!(body["stream_options"], json!({"include_usage": true}));

    assert!(from_str("deepseek:deepseek-chat").is_ok());
    assert!(matches!(from_str("deepseek:"), Err(Error::Config(_))));
}

#[test]
fn chunk_boundaries_never_change_the_text() {
    // DeepSeek sends keep-alive comments and plain `\n` line endings.
    let sse = concat!(
        ": keep-alive\n\n",
        r#"data: {"id":"x","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}"#,
        "\n\n",
        r#"data: {"id":"x","choices":[{"index":0,"delta":{"content":"Hello! How can"},"finish_reason":null}]}"#,
        "\n\n",
        r#"data: {"id":"x","choices":[{"index":0,"delta":{"content":" I help you? ✓"},"finish_reason":null}]}"#,
        "\n\n",
        r#"data: {"id":"x","choices":[{"index":0,"delta":{"content":""},"finish_reason":"stop"}],"usage":{"prompt_tokens":5,"completion_tokens":7}}"#,
        "\n\n",
        "data: [DONE]\n\n",
    );
    let bytes = sse.as_bytes();
    for split in 1..bytes.len() {
        let pieces: Vec<std::result::Result<Vec<u8>, Infallible>> =
            vec![Ok(bytes[..split].to_vec()), Ok(bytes[split..].to_vec())];
        let text: String = block_on(parse_stream(stream::iter(pieces)).collect::<Vec<_>>())
            .into_iter()
            .map(|c| c.unwrap().content)
            .collect();
        assert_eq!(
            text, "Hello! How can I help you? ✓",
            "split at byte {split}"
        );
    }
}

#[cfg(feature = "transport-tokio")]
mod tokio_transport {
    use futures::StreamExt;
    use orko_providers::{TokioTransport, Transport};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::Duration;

    #[tokio::test]
    async fn plain_http_round_trip_on_tokio() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0u8; 4096];
            let _ = s.read(&mut buf).unwrap();
            let body = "data: [DONE]\n\n";
            write!(
                s,
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        let t = TokioTransport::new().unwrap();
        let url = format!("http://{addr}/chat/completions");
        let resp = t.post(&url, &[], b"{}".to_vec()).await.unwrap();
        assert_eq!(resp.status, 200);
        let body: Vec<u8> = resp.body.map(|b| b.unwrap()).concat().await;
        assert_eq!(body, b"data: [DONE]\n\n");
    }

    #[tokio::test]
    async fn silent_server_times_out_on_tokio() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let _hold = std::thread::spawn(move || {
            listener.accept().map(|(s, _)| {
                std::thread::sleep(Duration::from_secs(5));
                drop(s);
            })
        });
        let t = TokioTransport::new()
            .unwrap()
            .with_timeout(Duration::from_millis(200));
        let err = t
            .post(&format!("http://{addr}/"), &[], Vec::new())
            .await
            .err()
            .unwrap();
        assert!(err.to_string().contains("timed out"), "{err}");
    }
}
