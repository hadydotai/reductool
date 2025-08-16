#[reductool::aitool]
/// Add two numbers
fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[reductool::aitool]
/// Greet a person by name; defaults to "Guest" when not provided.
fn greet(name: Option<String>) -> String {
    format!("Hello, {}!", name.unwrap_or_else(|| "Guest".to_string()))
}

fn main() {
    let schema = reductool::tools_to_schema();
    println!(
        "Tools schema:\n{}",
        serde_json::to_string_pretty(&schema).unwrap()
    );

    let (add_res, greet_res, greet2_res) = futures::executor::block_on(async {
        let add = reductool::dispatch_tool("add", serde_json::json!({ "a": 2, "b": 3 }))
            .await
            .expect("add failed");
        let greet = reductool::dispatch_tool("greet", serde_json::json!({ "name": "Hady" }))
            .await
            .expect("greet failed");
        let greet2 = reductool::dispatch_tool("greet", serde_json::json!({}))
            .await
            .expect("greet (no name) failed");
        (add, greet, greet2)
    });

    println!("add(2, 3) -> {}", add_res);
    println!("greet(name = \"Hady\") -> {}", greet_res);
    println!("greet() -> {}", greet2_res);
}
