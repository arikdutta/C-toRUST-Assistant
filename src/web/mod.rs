// Web UI for the migration assistant: the axum router and its handlers.

mod handler;
mod routes;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub use routes::app;

#[derive(Clone)]
pub struct AppState {
    pub jobs: Arc<Mutex<HashMap<String, String>>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
