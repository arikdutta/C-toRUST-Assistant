# csharp_rust_migrator

![Test](https://img.shields.io/badge/tests-passing-brightgreen) ![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg) ![Crates.io](https://img.shields.io/crates/v/csharp_rust_migrator) ![Downloads](https://img.shields.io/crates/d/csharp_rust_migrator)

A C# → Rust migration assistant. It parses a C# source file with Tree-sitter, extracts each declaration as a unit, translates each one to Rust via an LLM (with a running symbol table for context), and verifies the assembled result with a `cargo check` repair loop. Run with no arguments for a web UI on http://localhost:3000; run with a file path for CLI mode.

With no `ANTHROPIC_API_KEY` set, a built-in mock translator runs so the pipeline works offline.


<img width="1902" height="898" alt="C# to RUST Converter" src="https://github.com/user-attachments/assets/bcd358e3-8bbd-40cc-852e-1adb9fe5cc90" />


## Tech Stack

| Layer | Crate | Purpose |
|---|---|---|
| Web server | `axum 0.7` | HTTP routes and multipart file upload |
| Async runtime | `tokio 1` (full) | Async I/O and task scheduling |
| C# parsing | `tree-sitter 0.25` + `tree-sitter-c-sharp 0.23` | Grammar-based AST extraction |
| LLM client | `ureq 2` (json) | Synchronous HTTP calls to the Anthropic API |
| Serialization | `serde_json 1` | JSON request/response handling |
| Streaming | `tokio-stream 0.1` | Async event streaming |

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
