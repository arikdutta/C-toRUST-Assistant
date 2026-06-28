# csharp_rust_migrator

![Test](https://img.shields.io/badge/tests-passing-brightgreen) ![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg) ![Crates.io](https://img.shields.io/crates/v/csharp_rust_migrator) ![Downloads](https://img.shields.io/crates/d/csharp_rust_migrator)

A scalable C#-to-Rust translation pipeline. Tree-sitter handles **structure**
(splitting source into one translatable unit at a time); an LLM handles
**meaning** (the actual translation, including ownership, LINQ, async); and
`cargo check` **grades** the output, feeding compiler errors back into the model
until the code compiles.

This scales to real repositories because the LLM never sees more than one
declaration at a time, and every unit is compiler-verified rather than trusted.

With no `ANTHROPIC_API_KEY` set, a built-in mock translator runs so the pipeline works offline.

<img width="1902" height="898" alt="C__to_RUST_Converter_compressed" src="https://github.com/user-attachments/assets/78ef6e36-a3c9-4c59-88be-105bcc97c29c" />

## Tech Stack

| Layer | Crate | Purpose |
|---|---|---|
| Web server | `axum 0.7` | HTTP routes and multipart file upload |
| Async runtime | `tokio 1` (full) | Async I/O and task scheduling |
| C# parsing | `tree-sitter 0.25` + `tree-sitter-c-sharp 0.23` | Grammar-based AST extraction |
| LLM client | `ureq 2` (json) | Synchronous HTTP calls to the Anthropic API |
| Serialization | `serde_json 1` | JSON request/response handling |
| Streaming | `tokio-stream 0.1` | Async event streaming |

## How It Works

| Step | File | What it does |
|---|---|---|
| 1. Parse C# with Tree-sitter | `extractor.rs` | Finds class/struct/enum/interface boundaries. No translation — structure only. |
| 2. One node → LLM → Rust | `llm.rs` | Translates a single unit, given a running symbol table of already-translated signatures for compatibility. |
| 3. Assemble & orchestrate | `converter.rs` | Maintains the symbol table across units and drives the per-unit translation pass. |
| 4. `cargo check` → feed errors back | `cargo_check.rs` | Runs the compiler; on failure, re-prompts the LLM with the errors and retries up to MAX_REPAIRS. |

## Features

- Parses C# source files using the `tree-sitter-c-sharp` grammar — handles nested, multiline, and namespaced declarations correctly.
- Extracts classes, structs, enums, and interfaces as discrete translation units.
- Maintains a running symbol table across units so later translations have full context.
- Translates each unit via an LLM (Claude) with a `cargo check` repair loop for valid output.
- Maps primitive types automatically (`int`→`i32`, `string`→`String`, `bool`→`bool`, `double`→`f64`, `void`→`()`, etc.).
- Provides both a web UI (http://localhost:3000) and a CLI mode.
- Works offline without an API key using a built-in mock translator.

## Concepts Covered

| C# | Rust |
|---|---|
| `enum` | `enum` |
| `interface` | `trait` |
| `class` | `struct` + `impl` |
| property (`{ get; set; }`) | `pub` struct field |
| method | `fn` signature with `todo!()` |

## Roadmap

- **Interactive CLI**: Interactive prompts for users who prefer not to pass command-line arguments directly.
- **Incremental Parsing**: Re-parse only files that have changed since the last run, improving performance for large codebases.
- **Multiple Output Formats**: Generate output in HTML, PDF, and Markdown.
- **Cross-file Dependency Ordering**: Resolve type dependencies across files before translation.
- **Per-module Output**: Write each translated module to its own `.rs` file.
- **Per-unit Repair**: Run `cargo check` repair per translation unit rather than on the assembled output.
- **Configuration File**: Define settings in a TOML or JSON config file.
- **CI/CD Integration**: GitHub Actions workflow for automated migration on code changes.
- **Documentation Coverage Report**: Report which C# declarations were translated vs. skipped.

## Getting Started

### Prerequisites

- Rust (stable) and Cargo
- `ANTHROPIC_API_KEY` environment variable (optional — mock translator runs without it)

### Build

```bash
cargo build --release
```

## Usage

### Web UI

```bash
cargo run --release
# Open http://localhost:3000 in your browser and upload a .cs file
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

## Project Structure

```
.
├── src
│   ├── main.rs            # Entry point — CLI vs web dispatch
│   ├── extractor.rs       # Tree-sitter C# AST extraction
│   ├── converter.rs       # Symbol table and translation orchestration
│   ├── llm.rs             # Anthropic API client + mock translator
│   ├── cargo_check.rs     # cargo check repair loop
│   ├── web
│   │   ├── mod.rs
│   │   ├── handler.rs     # Upload and translation handlers
│   │   └── routes.rs      # Axum route definitions
│   ├── index.html         # Web UI
│   ├── style.css
│   └── result.html        # Translation result page
├── Cargo.toml
└── README.md
```

## Limitations

Translation quality depends on the LLM. Without an API key the built-in mock only covers the bundled sample inputs. See `tree-sitter-c-sharp-rust/README.md` → *Extending toward production* for cross-file dependency ordering, per-module output, and per-unit repair.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.

## License

MIT — see the [LICENSE](LICENSE) file for details.
