# csharp_rust_migrator

A C# → Rust migration assistant. It parses a C# source file with Tree-sitter,
extracts each declaration as a unit, translates each one to Rust via an LLM
(with a running symbol table for context), and verifies the assembled result
with a `cargo check` repair loop. Run with no arguments for a web UI on
http://localhost:3000; run with a file path for CLI mode.

See [`tree-sitter-c-sharp-rust/README.md`](tree-sitter-c-sharp-rust/README.md)
for the pipeline details. With no `ANTHROPIC_API_KEY` set, a built-in mock
translator runs so the pipeline works offline.

## Concepts covered

| C#                         | Rust                         |
|----------------------------|------------------------------|
| `enum`                     | `enum`                       |
| `interface`                | `trait`                      |
| `class`                    | `struct` + `impl`            |
| property (`{ get; set; }`) | `pub` struct field           |
| method                     | `fn` signature with `todo!()`|

Primitive types are mapped too (`int`→`i32`, `string`→`String`, `bool`→`bool`,
`double`→`f64`, `void`→`()`, etc.). User-defined types pass through unchanged.

## Build

```bash
cargo build --release
```

## Usage

```bash
cargo run --release -- examples/input.cs > output.rs
# or, after building:
./target/release/migrator examples/input.cs > output.rs
```

## Example

Input (`examples/input.cs`):

```csharp
enum Status { Active, Inactive, Pending }

interface IGreeter {
    string Greet(string name);
    void Reset();
}

class User {
    public int Id { get; set; }
    public string Name { get; set; }
    public bool IsActive { get; set; }

    public string Greet(string greeting) { }
    public int AddPoints(int amount) { }
}
```

Output (`examples/output.rs`):

```rust
#[derive(Debug, Clone)]
pub enum Status {
    Active,
    Inactive,
    Pending,
}

pub trait IGreeter {
    fn Greet(&self, name: String) -> String;
    fn Reset(&self);
}

#[derive(Debug, Default)]
pub struct User {
    pub Id: i32,
    pub Name: String,
    pub IsActive: bool,
}

impl User {
    pub fn Greet(&self, greeting: String) -> String {
        todo!()
    }
    pub fn AddPoints(&self, amount: i32) -> i32 {
        todo!()
    }
}
```

## Limitations

Parsing is handled by the `tree-sitter-c-sharp` grammar, so nested, multiline,
and namespaced declarations are extracted correctly. Translation quality depends
on the LLM; without an API key the built-in mock only covers the bundled sample
inputs. See `tree-sitter-c-sharp-rust/README.md` → *Extending toward production*
for cross-file dependency ordering, per-module output, and per-unit repair.

## License

MIT
