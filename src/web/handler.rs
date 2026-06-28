// Request handlers for the web UI.

use std::convert::Infallible;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse},
    response::sse::{Event, KeepAlive, Sse},
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt as _;

use crate::converter;
use super::AppState;

static INDEX_HTML: &str = include_str!("../index.html");
static RESULT_HTML: &str = include_str!("../result.html");
static CSS: &str = include_str!("../style.css");

pub async fn stylesheet() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css")], CSS)
}

pub async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

pub async fn convert(
    State(state): State<AppState>,
    mut mp: Multipart,
) -> impl IntoResponse {
    let mut src = String::new();
    let mut name = String::from("file.cs");

    while let Ok(Some(field)) = mp.next_field().await {
        if field.name() == Some("file") {
            name = field.file_name().unwrap_or("file.cs").to_string();
            let bytes = field.bytes().await.unwrap_or_default();
            src = String::from_utf8_lossy(&bytes).into_owned();
        }
    }

    let (tx, rx) = mpsc::unbounded_channel::<String>();

    tokio::task::spawn_blocking(move || {
        let output = converter::migrate_with_progress(&src, &tx);

        let html = RESULT_HTML
            .replace("{{NAME}}", &html_escape(&name))
            .replace("{{SRC}}", &html_escape(&src))
            .replace("{{OUTPUT}}", &html_escape(&output));

        let id = job_id();
        {
            let mut jobs = state.jobs.lock().unwrap();
            jobs.insert(id.clone(), html);
        }

        let payload = serde_json::json!({"e": "done", "id": id}).to_string();
        let _ = tx.send(payload);
        // tx dropped here — stream ends
    });

    let stream = UnboundedReceiverStream::new(rx)
        .map(|json| Ok::<_, Infallible>(Event::default().data(json)));

    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub async fn result_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let html = {
        let mut jobs = state.jobs.lock().unwrap();
        jobs.remove(&id)
    };
    match html {
        Some(h) => Html(h).into_response(),
        None => (StatusCode::NOT_FOUND, Html("<p>Result not found or expired.</p>")).into_response(),
    }
}

fn job_id() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
