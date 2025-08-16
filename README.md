# Reductool

<img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/hadydotai/reductool/build.yml?branch=main&style=for-the-badge">

This will let you turn Rust functions into LLM tools through an attribute macro. Here's a quick example

```rust
use reductool::aitool;

#[aitool]
/// Add allows you to add two numbers
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```



## Supported parameter types

The `#[aitool]` macro inspects function parameter types and emits a JSON Schema for each one. The following types are recognized specially; everything else falls back to `"type": "string"` in the schema and must implement `serde::Deserialize` to compile.

| Rust type                                      | JSON Schema emitted                                                                 | Required?                         | Notes |
|-----------------------------------------------|--------------------------------------------------------------------------------------|-----------------------------------|-------|
| i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize | { "type": "integer" }                                                               | Yes, unless wrapped in `Option`   |  |
| f32, f64                                       | { "type": "number" }                                                                 | Yes, unless wrapped in `Option`   |  |
| bool                                           | { "type": "boolean" }                                                                | Yes, unless wrapped in `Option`   |  |
| String                                         | { "type": "string" }                                                                 | Yes, unless wrapped in `Option`   | Prefer `String` over `&str` for deserialization. |
| [T; N]                                         | { "type": "array", "items": schema(T) }                                              | Yes, unless wrapped in `Option`   | At runtime, JSON must be an array of length exactly N; the schema does not encode this length constraint. |
| (T1, T2, ..., Tn)                              | { "type": "array", "items": [schema(T1), …, schema(Tn)], "minItems": n, "maxItems": n } | Yes, unless wrapped in `Option`   | Fixed-length, heterogeneous tuple. |
| Vec<T>                                   | { "type": "array", "items": schema(T) }                                              | Yes, unless wrapped in `Option`   | If T is unrecognized, items default to `{ "type": "string" }`. |
| Option<T>                                | schema(T)                                                                            | No (omitted from "required")      | Treated as optional; the schema does not add `"null"` type. Omit the field to pass `None`. |
| serde_json::Value (or a type named `Value`)    | {}                                                                                   | Yes, unless wrapped in `Option`   | Accepts any JSON value. The detection also matches a bare `Value` ident. |
| Other path types (custom structs/enums, etc.)  | { "type": "string" }                                                                  | Yes, unless wrapped in `Option`   | Must implement `serde::Deserialize` to compile; schema may not reflect true shape. |
| &T (references)                                | schema(T)                                                                            | —                                 | The schema normalizes `&T` to `T`, but the generated args struct uses the exact type; borrowed fields like `&str` generally cannot derive `Deserialize` here. Use owned types (e.g., `String`) instead. |

Notes and constraints:
- Parameters must be simple identifiers like `arg: T`. Patterns such as `(_: T)`, `(a, b): (T, U)`, or destructuring are rejected at compile time.
- Methods with a receiver (e.g., `self`, `&self`) are not supported; annotate free functions only.
- Required vs optional is determined solely by whether the type is `Option<T>`. All non-`Option` params are marked required.
- The example in `crates/reductool/examples/basic.rs` demonstrates both a required-arg function (`add(i32, i32)`) and an optional-arg function (`greet(Option<String>)`).

# License

Reductool
Copyright (C) 2025 hadydotai

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as published by
the Free Software Foundation, either version 3 of the License, or any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU Affero General Public License for more details.

You should have received a [copy](./LICENSE) of the GNU Affero General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
