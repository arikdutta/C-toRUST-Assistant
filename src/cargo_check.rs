// cargo_check.rs — Assemble translated snippets into a Cargo project and run
// `cargo check`, returning structured pass/fail + error text for the repair loop.
//
// If cargo isn't installed, `cargo_available()` reports that so the orchestrator
// can fall back to a dry run. Ported from the former
// `tree-sitter-c-sharp-rust/cargo.py`.

use std::path::Path;
use std::process::Command;

const CARGO_TOML: &str = "[package]
name = \"transpiled\"
version = \"0.1.0\"
edition = \"2021\"

[[bin]]
name = \"transpiled\"
path = \"src/main.rs\"
";

pub fn cargo_available() -> bool {
    Command::new("cargo")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Write Cargo.toml and src/main.rs from the assembled units.
pub fn write_project(root: &Path, rust_units: &[String]) -> std::io::Result<()> {
    let src_dir = root.join("src");
    std::fs::create_dir_all(&src_dir)?;
    std::fs::write(root.join("Cargo.toml"), CARGO_TOML)?;

    let body = rust_units.join("\n\n");
    // A main() so the crate is a runnable bin; #![allow(dead_code)] keeps cargo
    // check focused on real errors, not unused-warnings noise.
    let main_rs = format!(
        "#![allow(dead_code)]\n\n{body}\n\nfn main() {{\n    // entry point for the transpiled crate\n}}\n"
    );
    std::fs::write(src_dir.join("main.rs"), main_rs)?;
    Ok(())
}

/// Run `cargo check`. Returns (ok, error_text). `error_text` is empty on success,
/// or the compiler's error output on failure.
pub fn run_check(root: &Path) -> (bool, String) {
    let output = Command::new("cargo")
        .args(["check", "--message-format=short"])
        .current_dir(root)
        .output();

    match output {
        Ok(o) if o.status.success() => (true, String::new()),
        // Compiler errors land on stderr.
        Ok(o) => (false, String::from_utf8_lossy(&o.stderr).trim().to_string()),
        Err(e) => (false, format!("could not run cargo: {e}")),
    }
}
