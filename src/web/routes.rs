// Maps URL paths to the handlers in `super::handler`.

use axum::{
    routing::{get, post},
    Router,
};

use super::{handler, AppState};

pub fn app() -> Router {
    let state = AppState::new();
    Router::new()
        .route("/", get(handler::index))
        .route("/convert", post(handler::convert))
        .route("/result/:id", get(handler::result_by_id))
        .route("/style.css", get(handler::stylesheet))
        .with_state(state)
}
