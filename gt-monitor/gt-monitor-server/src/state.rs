use std::sync::Arc;

use gt_monitor_core::Engine;

/// Shared application state holding the query engine.
#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<Engine>,
}

impl AppState {
    pub fn new(engine: Engine) -> Self {
        AppState {
            engine: Arc::new(engine),
        }
    }
}
