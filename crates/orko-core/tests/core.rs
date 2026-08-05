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
    let fast = base.clone().with_model("mini").build();
    let smart = base.with_model("maxi").build();

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
                            && m.content == r#"[tool_call:c1] double({"x":3})"#
                    }));
                    assert!(request
                        .messages
                        .iter()
                        .any(|m| m.role == Role::Tool && m.content == "[double:c1] 6"));
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
    .build();

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

    let agent = create_agent(Refuser).build();
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
        .build();
    let err = futures::executor::block_on(agent.invoke("hi")).unwrap_err();
    assert!(matches!(err, Error::Other(_)), "got: {err}");
}

#[test]
fn static_router_ignores_request() {
    let router = StaticRouter::new("gpt-4o-mini");
    let req = CompletionRequest::new(vec![Message::user("x")]);
    assert_eq!(router.route(&req), "gpt-4o-mini");
}
