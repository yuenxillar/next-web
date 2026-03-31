use std::sync::OnceLock;

use tokio::sync::broadcast::Sender;

/// The graceful shutdown signal
pub static APPLICATION_GRACEFUL_SHUTDOWN_SIGNAL: OnceLock<Sender<()>> = OnceLock::new();
