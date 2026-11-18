//! Behavioral expansion tests for `#[tool]`: spec generation and dispatch.
//! Tool subjects live in `fixtures.rs`; calls are driven with
//! `Tool::call_blocking`, so no async runtime is involved.

mod fixtures;

use fixtures::*;
use orko_core::Error;
use serde_json::json;

#[test]
fn spec_comes_from_signature_desc_and_docs() {
    let t = get_weather();
    assert_eq!(t.name(), "get_weather");
    assert_eq!(t.description(), "Get the weather for a city.");
    assert_eq!(
        t.parameters_schema(),
        json!({"type":"object","properties":{"city":{"type":"string"}},"required":["city"]})
    );
    // No `desc`: the doc comment is the description.
    assert_eq!(greet().description(), "Greet someone.\n\nPolitely.");
    // `desc = "..."` wins over the doc comment.
    assert_eq!(farewell().description(), "Say goodbye.");
}

#[test]
fn schema_type_mapping() {
    assert_eq!(
        add().parameters_schema(),
        json!({
            "type":"object",
            "properties":{"a":{"type":"integer"},"b":{"type":"number"}},
            "required":["a","b"]
        })
    );
    assert_eq!(
        sum().parameters_schema(),
        json!({
            "type":"object",
            "properties":{"xs":{"type":"array","items":{"type":"integer"}}},
            "required":["xs"]
        })
    );
    // Zero params: bare object schema — no `properties` or `required` keys.
    assert_eq!(hello().parameters_schema(), json!({"type":"object"}));
    // Unknown type falls back to the open schema.
    assert_eq!(
        pretty().parameters_schema(),
        json!({"type":"object","properties":{"value":{}},"required":["value"]})
    );
}

#[test]
fn call_runs_the_function() {
    assert_eq!(
        get_weather().call_blocking(json!({"city":"SF"})).unwrap(),
        "sunny in SF"
    );
    assert_eq!(add().call_blocking(json!({"a":1,"b":2.5})).unwrap(), "3.5");
    assert_eq!(sum().call_blocking(json!({"xs":[1,2,3]})).unwrap(), "6");
    assert_eq!(
        pretty().call_blocking(json!({"value":{"k":1}})).unwrap(),
        r#"{"k":1}"#
    );
}

#[test]
fn async_fns_are_supported() {
    assert_eq!(echo().call_blocking(json!({"text":"hi"})).unwrap(), "hi");
}

#[test]
fn optional_params_may_be_omitted() {
    let t = greet();
    assert_eq!(t.parameters_schema()["required"], json!(["name"]));
    assert_eq!(t.call_blocking(json!({"name":"Sam"})).unwrap(), "hello Sam");
    assert_eq!(
        t.call_blocking(json!({"name":"Sam","greeting":"yo"}))
            .unwrap(),
        "yo Sam"
    );
}

#[test]
fn bad_args_surface_as_tool_execution() {
    // Wrong type — the message names the argument so the model can self-correct.
    let err = get_weather().call_blocking(json!({"city":42})).unwrap_err();
    assert!(matches!(err, Error::ToolExecution(_)), "{err}");
    assert!(err.to_string().contains("city"), "{err}");
    // Missing required argument.
    let err = get_weather().call_blocking(json!({})).unwrap_err();
    assert!(matches!(err, Error::ToolExecution(_)), "{err}");
    // Not an object at all.
    let err = get_weather().call_blocking(json!("nope")).unwrap_err();
    assert!(matches!(err, Error::ToolExecution(_)), "{err}");
}
