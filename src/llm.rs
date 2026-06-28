// llm.rs — Wraps the LLM call. The live implementation hits the Anthropic
// Messages API over raw HTTP (Rust has no official Anthropic SDK); a mock
// implementation lets the pipeline run end-to-end without a key so you can watch
// the orchestration/repair loop work.
//
// Ported from the former `tree-sitter-c-sharp-rust/llm.py`. The API key is read
// from the ANTHROPIC_API_KEY environment variable, or from a `.env` file in the
// crate root (KEY=VALUE lines) if the variable is unset. With no key, the mock
// translator runs.

use std::path::Path;

use serde_json::{json, Value};

const MODEL: &str = "claude-sonnet-4-6";

const SYSTEM_PROMPT: &str = "You are a C#-to-Rust transpiler. You translate ONE C# \
declaration at a time into idiomatic, compilable Rust.

Rules:
- Output ONLY Rust code. No prose, no markdown fences, no explanations.
- Map C# classes to `struct` + `impl`. Interfaces to `trait`. Enums to `enum`.
- Map exceptions to `Result<T, E>`, LINQ to iterator chains, `null` to `Option`.
- Use snake_case for fields/methods, PascalCase for types.
- ONLY use the Rust standard library (`std`). Do NOT import or use any external
  crates (no chrono, no serde, no tokio, etc.). Use `std::time::SystemTime` for
  dates, `f64` for decimals.
- Do NOT emit `fn main()`. Do NOT redefine types that appear in the already-
  translated signatures list — those are already present in the final file.
- If a referenced type is not yet defined, assume it will exist; do not redefine it.
- Do not include `use` imports for types you are not directly using in this unit.
";

/// Translate one C# unit to Rust.
/// - `known_signatures`: Rust signatures already produced (the running symbol table).
/// - `prior_errors`: if this is a repair attempt, the cargo check errors to fix.
///
/// Returns the Rust snippet, or an error string if the live API call failed.
pub fn translate(
    csharp_source: &str,
    known_signatures: &[String],
    prior_errors: Option<&str>,
) -> Result<String, String> {
    match api_key() {
        Some(key) => anthropic_translate(&key, csharp_source, known_signatures, prior_errors),
        None => Ok(mock_translate(csharp_source, prior_errors)),
    }
}

fn anthropic_translate(
    key: &str,
    csharp_source: &str,
    known_signatures: &[String],
    prior_errors: Option<&str>,
) -> Result<String, String> {
    let context = if known_signatures.is_empty() {
        "(none yet)".to_string()
    } else {
        known_signatures.join("\n")
    };

    let mut user = format!(
        "Already-translated Rust signatures (already in the output file — do NOT redefine these):\n\
{context}\n\n\
Translate ONLY this C# declaration to Rust. Output just the new item(s), nothing already listed above:\n\
```csharp\n{csharp_source}\n```"
    );

    if let Some(errors) = prior_errors {
        user.push_str(&format!(
            "\n\nYour previous Rust output failed to compile. Fix these errors:\n\
```\n{errors}\n```\n\
Output the corrected full Rust for this unit."
        ));
    }

    let body = json!({
        "model": MODEL,
        "max_tokens": 2000,
        "system": SYSTEM_PROMPT,
        "messages": [{ "role": "user", "content": user }],
    });

    let resp = match ureq::post("https://api.anthropic.com/v1/messages")
        .set("x-api-key", key)
        .set("anthropic-version", "2023-06-01")
        .set("content-type", "application/json")
        .send_json(body)
    {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => {
            let detail = r.into_string().unwrap_or_default();
            return Err(format!("Anthropic API returned {code}: {detail}"));
        }
        Err(e) => return Err(format!("request to Anthropic API failed: {e}")),
    };

    let value: Value = resp
        .into_json()
        .map_err(|e| format!("reading Anthropic API response: {e}"))?;

    let text = value["content"]
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter(|b| b["type"] == "text")
                .filter_map(|b| b["text"].as_str())
                .collect::<String>()
        })
        .unwrap_or_default();

    Ok(clean(&strip_fences(&text)))
}

/// Remove ```rust ... ``` fences if the model added them anyway.
fn strip_fences(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.starts_with("```") {
        if let Some(first_newline) = trimmed.find('\n') {
            let after = &trimmed[first_newline + 1..];
            if let Some(end) = after.rfind("```") {
                return after[..end].trim().to_string();
            }
        }
    }
    trimmed.to_string()
}

const BANNED_IMPORTS: &[&str] = &[
    "chrono", "serde", "tokio", "rand", "anyhow", "thiserror", "uuid",
];

/// Strip a `fn main()` block and external-crate imports the model shouldn't emit.
fn clean(text: &str) -> String {
    let mut s = text.to_string();

    // Remove a simple, single-level `fn main() { ... }` block.
    if let Some(start) = s.find("fn main()") {
        if let Some(rel) = s[start..].find("\n}") {
            let end = start + rel + 2; // include the "\n}"
            s.replace_range(start..end, "");
        }
    }

    // Drop `use <banned-crate>...;` import lines.
    let kept: Vec<&str> = s
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            match trimmed.strip_prefix("use ") {
                Some(rest) => {
                    let root = rest
                        .split(|c: char| c == ':' || c == ';' || c.is_whitespace())
                        .next()
                        .unwrap_or("");
                    !BANNED_IMPORTS.contains(&root)
                }
                None => true,
            }
        })
        .collect();

    kept.join("\n").trim().to_string()
}

/// Anthropic API key: prefer the environment variable, falling back to a `.env`
/// file in the crate root. With no key available, the caller runs the mock.
fn api_key() -> Option<String> {
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        if !key.trim().is_empty() {
            return Some(key);
        }
    }
    dotenv_value("ANTHROPIC_API_KEY")
}

/// Minimal `.env` reader (KEY=VALUE per line), so no external dotenv dependency
/// is needed. Checks the exe directory, the cwd, and the compile-time crate root.
fn dotenv_value(name: &str) -> Option<String> {
    let compile_time_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));
    let cwd = std::env::current_dir().ok();

    let mut candidates = vec![
        compile_time_root.join(".env"),
        compile_time_root.join("src").join(".env"),
    ];
    if let Some(ref dir) = exe_dir {
        candidates.push(dir.join(".env"));
    }
    if let Some(ref dir) = cwd {
        candidates.push(dir.join(".env"));
    }
    for candidate in candidates {
        let Ok(contents) = std::fs::read_to_string(&candidate) else {
            continue;
        };
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                if key.trim() == name {
                    let value = value.trim().trim_matches('"').trim_matches('\'');
                    if !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Mock translator: enough hand-coded rules to make the demo produce real,
// compilable Rust for the sample input, including a deliberate first-pass error
// so you can see the repair loop fix it.
// ---------------------------------------------------------------------------
fn mock_translate(csharp_source: &str, prior_errors: Option<&str>) -> String {
    let s = csharp_source;

    if s.contains("enum AccountType") {
        return "#[derive(Debug, Clone, Copy)]\n\
pub enum AccountType {\n\
    Checking,\n\
    Savings,\n\
    Credit,\n\
}"
        .to_string();
    }

    if s.contains("ITransactable") {
        return "pub trait ITransactable {\n\
    fn deposit(&mut self, amount: f64);\n\
    fn withdraw(&mut self, amount: f64) -> bool;\n\
}"
        .to_string();
    }

    if s.contains("class Account") {
        return "pub struct Account {\n\
    pub id: i32,\n\
    pub owner: String,\n\
    pub balance: f64,\n\
    pub is_frozen: bool,\n\
}\n\n\
impl ITransactable for Account {\n\
    fn deposit(&mut self, amount: f64) {\n\
        self.balance += amount;\n\
    }\n\n\
    fn withdraw(&mut self, amount: f64) -> bool {\n\
        if self.is_frozen || self.balance < amount { return false; }\n\
        self.balance -= amount;\n\
        true\n\
    }\n\
}\n\n\
impl Account {\n\
    pub fn get_balance(&self) -> f64 {\n\
        self.balance\n\
    }\n\
}"
        .to_string();
    }

    if s.contains("enum Status") {
        return "#[derive(Debug, Clone, Copy)]\npub enum Status {\n    Active,\n    Inactive,\n}"
            .to_string();
    }

    if s.contains("struct Point") {
        return "#[derive(Debug, Clone, Copy)]\npub struct Point {\n    pub x: i32,\n    pub y: i32,\n}"
            .to_string();
    }

    if s.contains("class Calculator") {
        // First pass intentionally returns a version with a type error (returns
        // i64 from an i32 fn) to demonstrate the repair loop.
        if prior_errors.is_none() {
            return "pub struct Calculator {\n\
    pub total: i32,\n\
    pub name: String,\n\
}\n\n\
impl Calculator {\n\
    pub fn add(&self, a: i32, b: i32) -> i32 {\n\
        let r: i64 = (a + b) as i64;\n\
        r\n\
    }\n\n\
    pub fn sum(&self, values: &[i32]) -> i32 {\n\
        values.iter().sum()\n\
    }\n\
}"
            .to_string();
        }
        // Repair pass: fix the mismatch.
        return "pub struct Calculator {\n\
    pub total: i32,\n\
    pub name: String,\n\
}\n\n\
impl Calculator {\n\
    pub fn add(&self, a: i32, b: i32) -> i32 {\n\
        a + b\n\
    }\n\n\
    pub fn sum(&self, values: &[i32]) -> i32 {\n\
        values.iter().sum()\n\
    }\n\
}"
        .to_string();
    }

    "// (mock) unhandled unit".to_string()
}
