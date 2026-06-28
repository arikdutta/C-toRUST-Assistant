// C# -> Rust conversion pipeline.
//
//   1. Tree-sitter extracts C# units (types first, then methods).
//   2. Each unit -> LLM -> Rust snippet, carrying a running symbol table as context.
//   3. Assemble all snippets into a Cargo project.
//   4. `cargo check`. If it fails, feed errors + current code back to the LLM
//      for each unit and retry, up to MAX_REPAIRS times.

use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::mpsc::UnboundedSender;

use crate::cargo_check;
use crate::extractor::CSharpExtractor;
use crate::llm;

const MAX_REPAIRS: usize = 3;

pub fn migrate(src: &str) -> String {
    run_pipeline(src, None)
}

pub fn migrate_with_progress(src: &str, tx: &UnboundedSender<String>) -> String {
    run_pipeline(src, Some(tx))
}

fn emit(tx: Option<&UnboundedSender<String>>, val: serde_json::Value) {
    if let Some(t) = tx {
        if let Ok(s) = serde_json::to_string(&val) {
            let _ = t.send(s);
        }
    }
}

fn run_pipeline(src: &str, tx: Option<&UnboundedSender<String>>) -> String {
    match orchestrate(src, tx) {
        Ok(rust) => rust,
        Err(e) => {
            emit(tx, serde_json::json!({"e": "error", "msg": e}));
            format!(
                "// C# -> Rust conversion failed:\n// {}\n",
                e.replace('\n', "\n// ")
            )
        }
    }
}

fn orchestrate(src: &str, tx: Option<&UnboundedSender<String>>) -> Result<String, String> {
    // STEP 1 — Tree-sitter extraction. Sort so types come before methods.
    let mut units = CSharpExtractor::new().parse(src);
    units.sort_by_key(|u| kind_order(&u.kind));
    eprintln!("STEP 1 — Tree-sitter extraction: {} unit(s)", units.len());
    for u in &units {
        eprintln!("  • {:12} [{}]", u.name, u.kind);
    }
    emit(tx, serde_json::json!({
        "e": "step1",
        "count": units.len(),
        "units": units.iter()
            .map(|u| serde_json::json!({"name": u.name, "kind": u.kind}))
            .collect::<Vec<_>>()
    }));

    // STEP 2 — Translate each unit (LLM), building a running symbol table.
    eprintln!("STEP 2 — Translate each unit (LLM)");
    let mut rust_units: Vec<String> = Vec::new();
    let mut known_sigs: Vec<String> = Vec::new();
    for u in &units {
        let rust = llm::translate(&u.source, &known_sigs, None)?;
        let new_sigs = extract_rust_signatures(&rust);
        eprintln!(
            "  • {:12} -> {} chars Rust, +{} signatures into context",
            u.name,
            rust.len(),
            new_sigs.len()
        );
        emit(tx, serde_json::json!({"e": "unit", "name": u.name, "chars": rust.len()}));
        known_sigs.extend(new_sigs);
        rust_units.push(rust);
    }

    // STEP 3 & 4 — Assemble a Cargo project + check/repair loop.
    eprintln!("STEP 3 & 4 — Assemble Cargo project + check/repair loop");
    emit(tx, serde_json::json!({"e": "step3"}));

    if cargo_check::cargo_available() {
        let workdir = std::env::temp_dir().join(format!("transpiled_{}", stamp()));
        for attempt in 1..=MAX_REPAIRS {
            cargo_check::write_project(&workdir, &rust_units)
                .map_err(|e| format!("writing cargo project: {e}"))?;
            let (ok, errors) = cargo_check::run_check(&workdir);
            if ok {
                eprintln!("  ✓ cargo check passed on attempt {attempt}");
                emit(tx, serde_json::json!({"e": "cargo", "ok": true, "attempt": attempt}));
                break;
            }
            eprintln!("  ✗ attempt {attempt}: cargo check failed");
            let is_last = attempt == MAX_REPAIRS;
            emit(tx, serde_json::json!({"e": "cargo", "ok": false, "attempt": attempt, "final": is_last}));
            if is_last {
                eprintln!("    giving up after max repairs.");
                break;
            }
            // Repair: re-translate each unit with the errors as feedback.
            let mut repaired: Vec<String> = Vec::new();
            let mut sigs: Vec<String> = Vec::new();
            for u in &units {
                let fixed = llm::translate(&u.source, &sigs, Some(&errors))?;
                sigs.extend(extract_rust_signatures(&fixed));
                repaired.push(fixed);
            }
            rust_units = repaired;
        }
        let _ = std::fs::remove_dir_all(&workdir);
    } else {
        eprintln!("  ⚠ cargo not installed — skipping live check/repair loop.");
        emit(tx, serde_json::json!({"e": "cargo_skip"}));
    }

    Ok(rust_module_body(&rust_units))
}

fn kind_order(kind: &str) -> u8 {
    match kind {
        "enum_declaration" => 0,
        "struct_declaration" | "record_declaration" => 1,
        "interface_declaration" => 2,
        "class_declaration" => 3,
        "method_declaration" => 4,
        _ => 99,
    }
}

fn extract_rust_signatures(rust: &str) -> Vec<String> {
    let lines: Vec<&str> = rust.lines().collect();
    let mut sigs: Vec<String> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let stripped = lines[i].trim();
        if stripped.starts_with("pub trait ")
            || stripped.starts_with("pub struct ")
            || stripped.starts_with("pub enum ")
            || stripped.starts_with("pub type ")
        {
            let mut block = vec![lines[i].to_string()];
            let mut depth = brace_delta(lines[i]);
            i += 1;
            while i < lines.len() && depth > 0 {
                block.push(lines[i].to_string());
                depth += brace_delta(lines[i]);
                i += 1;
            }
            sigs.push(block.join("\n"));
        } else if stripped.starts_with("pub fn ") {
            sigs.push(stripped.trim_end_matches('{').trim().to_string());
            i += 1;
        } else {
            i += 1;
        }
    }
    sigs
}

fn brace_delta(line: &str) -> i32 {
    line.matches('{').count() as i32 - line.matches('}').count() as i32
}

fn rust_module_body(rust_units: &[String]) -> String {
    rust_units.join("\n\n").trim().to_string()
}

fn stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}
