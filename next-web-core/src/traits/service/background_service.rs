use async_trait::async_trait;

#[async_trait]
pub trait BackgroundService
where
    Self: Send + Sync,
    Self: 'static,
{
    fn service_name(&self) -> &'static str;

    async fn run(&self) -> Result<(), ServiceError>;

    async fn shutdown(&self);

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
