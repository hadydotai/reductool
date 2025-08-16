use anyhow::Result;
use futures::future::BoxFuture;
use linkme::distributed_slice;
use serde_json::Value;

// Re-export the proc macro so we can simply do `use reductool::aitool;`
pub use reductool_proc_macro::aitool;

// Re-export linkme under a stable path for the generated macro code
pub use linkme as __linkme;

pub type InvokeFuture = BoxFuture<'static, Result<Value>>;

#[derive(Clone)]
pub struct ToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
    pub json_schema: &'static str,
    pub invoke: fn(Value) -> InvokeFuture,
}

#[distributed_slice]
pub static ALL_TOOLS: [ToolDefinition] = [..];

pub fn tools_to_schema() -> Value {
    Value::Array(
        ALL_TOOLS
            .iter()
            .map(|t| serde_json::from_str(t.json_schema).unwrap())
            .collect(),
    )
}

pub async fn dispatch_tool(name: &str, args: Value) -> Result<Value> {
    (ALL_TOOLS
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| anyhow::anyhow!("Unknown tool: {name}"))?
        .invoke)(args)
    .await
}
