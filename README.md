# Reductool - Allow Rust function to be called by LLMs

This will let you turn Rust functions into LLM tools through an attribute macro. Here's a quick example

```rust
use reductool::aitool;

#[aitool]
/// Add allows you to add two numbers
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

