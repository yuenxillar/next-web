use async_trait::async_trait;

/// A long-lived application component managed alongside the web server.
///
/// Implementations should keep `run` alive until their own shutdown mechanism
/// is signalled. `shutdown` should signal that mechanism and perform any
/// resource cleanup; the manager then waits for `run` to finish. A UDP server,
/// message consumer, or middleware support worker can all use this lifecycle.
#[async_trait]
pub trait BackgroundService
where
    Self: Send + Sync,
    Self: 'static,
{
    /// Returns a stable, unique name within one manager.
    fn name(&self) -> &'static str;

    /// Runs the service until its internal shutdown mechanism is signalled.
    ///
    /// Unexpected completion or an error causes the manager to restart it.
    async fn run(&self) -> Result<(), ServiceError>;

    /// Signals `run` to stop and releases resources owned by the service.
    ///
    /// The manager awaits this method while `run` is still alive, then waits
    /// for `run` to finish. Implementations should make repeated calls safe.
    async fn shutdown(&self);

    /// Reports service health. The manager calls this periodically as a
    /// heartbeat and restarts the service when it returns `Unhealthy`.
    async fn health_check(&self) -> HealthStatus;
}

#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Service initialization failed: {0}")]
    InitFailed(String),

    #[error("Service runtime error: {0}")]
    RuntimeError(String),

    #[error("Service already running")]
    AlreadyRunning,

    #[error("Service not found")]
    NotFound,

    #[error("Shutdown timeout")]
    ShutdownTimeout,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy(String),
    Starting,
    Stopping,
    Unknown,
}
