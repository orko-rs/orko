//! `#[tool]` test subjects covering every schema and description path.

use orko_macros::tool;

#[tool(desc = "Get the weather for a city.")]
pub fn get_weather(city: String) -> String {
    format!("sunny in {city}")
}

#[tool(desc = "Add two numbers.")]
pub fn add(a: i64, b: f64) -> f64 {
    a as f64 + b
}

#[tool(desc = "Say hello.")]
pub fn hello() -> String {
    "hello".to_owned()
}

#[tool(desc = "Echo text back, asynchronously.")]
pub async fn echo(text: String) -> String {
    text
}

/// Greet someone.
///
/// Politely.
#[tool]
pub fn greet(name: String, greeting: Option<String>) -> String {
    format!("{} {name}", greeting.unwrap_or_else(|| "hello".to_owned()))
}

/// Doc comment, which `desc` overrides.
#[tool(desc = "Say goodbye.")]
pub fn farewell(name: String) -> String {
    format!("bye {name}")
}

#[tool(desc = "Sum a list.")]
pub fn sum(xs: Vec<i64>) -> i64 {
    xs.iter().sum()
}

#[tool(desc = "Pretty-print arbitrary JSON.")]
pub fn pretty(value: serde_json::Value) -> String {
    value.to_string()
}
