#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy(String),
    Starting,
    Stopping,
    Unknown,
}
