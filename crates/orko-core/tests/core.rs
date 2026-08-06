use orko_core::*;

#[test]
fn message_roundtrips_through_json() {
    let msg = Message::user("hello");
    let json = serde_json::to_string(&msg).unwrap();
    assert_eq!(json, r#"{"role":"user","content":"hello"}"#);
    let back: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(back, msg);
}

#[test]
fn multipart_content_roundtrips_through_json() {
    let msg = Message::user(vec![
        ContentPart::text("what is in this image?"),
        ContentPart::image_url("https://example.com/cat.png"),
    ]);
    let json = serde_json::to_string(&msg).unwrap();
    assert_eq!(
        json,
        r#"{"role":"user","content":[{"type":"text","text":"what is in this image?"},{"type":"image","source":{"kind":"url","url":"https://example.com/cat.png"}}]}"#
    );
    let back: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(back, msg);
}

#[test]
fn file_content_part_roundtrips_through_json() {
    let msg = Message::user(vec![ContentPart::File {
        source: MediaSource::base64("application/pdf", "aGVsbG8="),
        name: Some("hello.pdf".into()),
    }]);
    let json = serde_json::to_string(&msg).unwrap();
    assert_eq!(
        json,
        r#"{"role":"user","content":[{"type":"file","source":{"kind":"base64","mime_type":"application/pdf","data":"aGVsbG8="},"name":"hello.pdf"}]}"#
    );
    let back: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(back, msg);
}

#[test]
fn content_push_part_promotes_text_to_parts() {
    let mut c: Content = "hi".into();
    c.push_part(ContentPart::image_url("https://example.com/cat.png"));
    let ContentView::Parts(parts) = c.pull() else {
        panic!("promoted to Parts");
    };
    assert_eq!(parts.len(), 2);
    assert!(matches!(&parts[0], ContentPart::Text { text } if text == "hi"));
    assert!(matches!(&parts[1], ContentPart::Image { .. }));

    c.push_part(ContentPart::text("more"));
    assert!(matches!(c.pull(), ContentView::Parts(p) if p.len() == 3));

    // Empty text is dropped on promotion, not kept as an empty part.
    let mut empty: Content = "".into();
    empty.push_part(ContentPart::image_url("https://example.com/cat.png"));
    assert!(matches!(empty.pull(), ContentView::Parts(p) if p.len() == 1));
}

#[test]
fn content_push_text_appends_by_variant() {
    // Onto Text: plain concatenation.
    let mut t: Content = "he".into();
    t.push_text("llo");
    assert_eq!(t.pull(), ContentView::Text("hello"));

    // Onto Parts: appended as a new text part.
    let mut p: Content = vec![ContentPart::image_url("https://example.com/cat.png")].into();
    p.push_text("caption");
    let ContentView::Parts(parts) = p.pull() else {
        panic!("still Parts");
    };
    assert_eq!(parts.len(), 2);
    assert!(matches!(&parts[1], ContentPart::Text { text } if text == "caption"));
}

#[test]
fn prompt_from_str_wraps_a_user_message() {
    let prompt: Prompt = "hi".into();
    assert_eq!(prompt.messages.len(), 1);
    assert_eq!(prompt.messages[0].role, Role::User);
}

#[test]
fn chunk_functional_update_construction() {
    let last = CompletionChunk {
        finish_reason: Some(FinishReason::ToolCalls),
        ..Default::default()
    };
    assert!(last.content.is_empty());
    assert!(last.tool_calls.is_empty());
    assert_eq!(last.finish_reason, Some(FinishReason::ToolCalls));
}

#[test]
fn cloned_builder_templates_multiple_agents() {
    struct EchoModel;
    impl Provider for EchoModel {
        async fn complete(&self, request: CompletionRequest) -> Result<CompletionStream> {
            let chunk = CompletionChunk {
                content: request.options.model.unwrap_or_default(),
                ..Default::default()
            };
            let stream: CompletionStream = Box::pin(futures::stream::iter([Ok(chunk)]));
            Ok(stream)
        }
    }

    let base = create_agent(std::sync::Arc::new(EchoModel)).with_system_prompt("terse");
    let fast = base.clone().with_model("mini").build().unwrap();
    let smart = base.with_model("maxi").build().unwrap();

    assert_eq!(
        futures::executor::block_on(fast.invoke("hi")).unwrap(),
        "mini"
    );
    assert_eq!(
        futures::executor::block_on(smart.invoke("hi")).unwrap(),
        "maxi"
    );
}

#[test]
fn invoke_runs_the_tool_call_loop() {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct Doubler;
    impl Tool for Doubler {
        fn name(&self) -> &str {
            "double"
        }
        fn description(&self) -> &str {
            "doubles x"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object", "properties": {"x": {"type": "integer"}}})
        }
        fn call<'a>(
            &'a self,
            args: serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
            Box::pin(async move { Ok((args["x"].as_i64().unwrap() * 2).to_string()) })
        }
    }

    // Never called; exists so the binary search has neighbors to get wrong.
    struct Named(&'static str);
    impl Tool for Named {
        fn name(&self) -> &str {
            self.0
        }
        fn description(&self) -> &str {
            ""
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }
        fn call<'a>(
            &'a self,
            _args: serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
            Box::pin(async { panic!("wrong tool dispatched") })
        }
    }

    // Turn 0: a tool call with `arguments` split across two fragments.
    // Turn 1: expects the fed-back tool result, answers in text.
    struct Scripted {
        turn: AtomicUsize,
    }
    impl Provider for Scripted {
        async fn complete(&self, request: CompletionRequest) -> Result<CompletionStream> {
            let chunks = match self.turn.fetch_add(1, Ordering::SeqCst) {
                0 => vec![
                    Ok(CompletionChunk {
                        tool_calls: vec![ToolCallDelta {
                            index: 0,
                            id: Some("c1".into()),
                            name: Some("double".into()),
                            arguments: r#"{"x":"#.into(),
                        }],
                        ..Default::default()
                    }),
                    Ok(CompletionChunk {
                        tool_calls: vec![ToolCallDelta {
                            index: 0,
                            arguments: "3}".into(),
                            ..Default::default()
                        }],
                        finish_reason: Some(FinishReason::ToolCalls),
                        ..Default::default()
                    }),
                ],
                _ => {
                    assert!(request.messages.iter().any(|m| {
                        m.role == Role::Assistant
                            && m.content.pull()
                                == ContentView::Text(r#"[tool_call:c1] double({"x":3})"#)
                    }));
                    assert!(request.messages.iter().any(|m| m.role == Role::Tool
                        && m.content.pull() == ContentView::Text("[double:c1] 6")));
                    vec![Ok(CompletionChunk {
                        content: "six".into(),
                        ..Default::default()
                    })]
                }
            };
            let stream: CompletionStream = Box::pin(futures::stream::iter(chunks));
            Ok(stream)
        }
    }

    let agent = create_agent(Scripted {
        turn: AtomicUsize::new(0),
    })
    .with_tools([
        Arc::new(Named("zeta")) as Arc<dyn Tool>,
        Arc::new(Named("alpha")),
    ])
    .with_tool(Arc::new(Doubler))
    .build()
    .unwrap();

    let names: Vec<_> = agent.tools().iter().map(|t| t.name()).collect();
    assert_eq!(names, ["alpha", "double", "zeta"]);
    assert_eq!(
        futures::executor::block_on(agent.invoke("hi")).unwrap(),
        "six"
    );
}

#[test]
fn refusal_only_turn_is_not_an_empty_success() {
    struct Refuser;
    impl Provider for Refuser {
        async fn complete(&self, _request: CompletionRequest) -> Result<CompletionStream> {
            let chunks = vec![Ok(CompletionChunk {
                refusal: Some("no can do".into()),
                ..Default::default()
            })];
            let stream: CompletionStream = Box::pin(futures::stream::iter(chunks));
            Ok(stream)
        }
    }

    let agent = create_agent(Refuser).build().unwrap();
    assert_eq!(
        futures::executor::block_on(agent.invoke("hi")).unwrap(),
        "no can do"
    );
}

#[test]
fn invoke_errors_when_turn_cap_exhausted() {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    struct Noop;
    impl Tool for Noop {
        fn name(&self) -> &str {
            "noop"
        }
        fn description(&self) -> &str {
            ""
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }
        fn call<'a>(
            &'a self,
            _args: serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
            Box::pin(async { Ok(String::new()) })
        }
    }

    struct AlwaysCalls;
    impl Provider for AlwaysCalls {
        async fn complete(&self, _request: CompletionRequest) -> Result<CompletionStream> {
            let chunks = vec![Ok(CompletionChunk {
                tool_calls: vec![ToolCallDelta {
                    index: 0,
                    name: Some("noop".into()),
                    arguments: "{}".into(),
                    ..Default::default()
                }],
                ..Default::default()
            })];
            let stream: CompletionStream = Box::pin(futures::stream::iter(chunks));
            Ok(stream)
        }
    }

    let agent = create_agent(AlwaysCalls)
        .with_tool(Arc::new(Noop))
        .with_max_tool_turns(2)
        .build()
        .unwrap();
    let err = futures::executor::block_on(agent.invoke("hi")).unwrap_err();
    assert!(matches!(err, Error::ToolCallDepth(_)), "got: {err}");
}

#[test]
fn tool_failures_feed_back_for_self_correction() {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    struct Boom;
    impl Tool for Boom {
        fn name(&self) -> &str {
            "boom"
        }
        fn description(&self) -> &str {
            ""
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }
        fn call<'a>(
            &'a self,
            _args: serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
            Box::pin(async { Err(Error::ToolExecution("kaboom".into())) })
        }
    }

    // Turn 0: three bad calls — unknown tool, malformed args, failing tool.
    // Turn 1: all three failures must be visible as tool results.
    struct Scripted {
        turn: AtomicUsize,
    }
    impl Provider for Scripted {
        async fn complete(&self, request: CompletionRequest) -> Result<CompletionStream> {
            let chunks = match self.turn.fetch_add(1, Ordering::SeqCst) {
                0 => vec![Ok(CompletionChunk {
                    tool_calls: vec![
                        ToolCallDelta {
                            index: 0,
                            name: Some("nope".into()),
                            arguments: "{}".into(),
                            ..Default::default()
                        },
                        ToolCallDelta {
                            index: 1,
                            name: Some("boom".into()),
                            arguments: "{".into(),
                            ..Default::default()
                        },
                        ToolCallDelta {
                            index: 2,
                            name: Some("boom".into()),
                            arguments: "{}".into(),
                            ..Default::default()
                        },
                    ],
                    finish_reason: Some(FinishReason::ToolCalls),
                    ..Default::default()
                })],
                _ => {
                    let tool_results: Vec<_> = request
                        .messages
                        .iter()
                        .filter(|m| m.role == Role::Tool)
                        .filter_map(|m| match m.content.pull() {
                            ContentView::Text(t) => Some(t),
                            ContentView::Parts(_) => None,
                        })
                        .collect();
                    assert!(tool_results
                        .iter()
                        .any(|c| c.contains("unknown tool `nope`")));
                    assert!(tool_results.iter().any(|c| c.contains("invalid arguments")));
                    assert!(tool_results
                        .iter()
                        .any(|c| c.contains("tool execution error: kaboom")));
                    vec![Ok(CompletionChunk {
                        content: "recovered".into(),
                        ..Default::default()
                    })]
                }
            };
            let stream: CompletionStream = Box::pin(futures::stream::iter(chunks));
            Ok(stream)
        }
    }

    let agent = create_agent(Scripted {
        turn: AtomicUsize::new(0),
    })
    .with_tool(Arc::new(Boom))
    .build()
    .unwrap();
    assert_eq!(
        futures::executor::block_on(agent.invoke("hi")).unwrap(),
        "recovered"
    );
}

#[test]
fn duplicate_tool_names_rejected_at_build() {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    struct Silent;
    impl Provider for Silent {
        async fn complete(&self, _request: CompletionRequest) -> Result<CompletionStream> {
            let stream: CompletionStream =
                Box::pin(futures::stream::iter([Ok(CompletionChunk::default())]));
            Ok(stream)
        }
    }

    struct Dup;
    impl Tool for Dup {
        fn name(&self) -> &str {
            "dup"
        }
        fn description(&self) -> &str {
            ""
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }
        fn call<'a>(
            &'a self,
            _args: serde_json::Value,
        ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
            Box::pin(async { Ok(String::new()) })
        }
    }

    let err = match create_agent(Silent)
        .with_tools([Arc::new(Dup) as Arc<dyn Tool>, Arc::new(Dup)])
        .build()
    {
        Ok(_) => panic!("duplicate tool names were accepted"),
        Err(e) => e,
    };
    assert!(matches!(err, Error::Config(_)), "got: {err}");
}

#[test]
fn static_router_ignores_request() {
    let router = StaticRouter::new("gpt-4o-mini");
    let req = CompletionRequest::new(vec![Message::user("x")]);
    assert_eq!(router.route(&req), "gpt-4o-mini");
}
