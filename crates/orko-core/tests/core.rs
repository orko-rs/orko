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
fn static_router_ignores_request() {
    let router = StaticRouter::new("gpt-4o-mini");
    let req = CompletionRequest::new(vec![Message::user("x")]);
    assert_eq!(router.route(&req), "gpt-4o-mini");
}
