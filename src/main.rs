// C# -> Rust migration assistant.
// No args: starts a web UI at http://localhost:3000
// With args: `migrator input.cs` prints converted Rust to stdout (CLI mode)

mod cargo_check;
mod converter;
mod extractor;
mod llm;
mod web;

use std::env;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    if let Some(path) = env::args().nth(1) {
        let src = std::fs::read_to_string(&path).expect("cannot read file");
        println!("{}", converter::migrate(&src));
        return;
    }

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    eprintln!("Listening on http://localhost:3000");
    axum::serve(listener, web::app()).await.unwrap();
}
