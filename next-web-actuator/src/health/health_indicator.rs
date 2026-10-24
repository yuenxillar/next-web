use async_trait::async_trait;

use crate::health::Health;

/// Strategy trait used to contribute Health to the results returned from the reactive variant of the
/// HealthEndpoint.
#[async_trait]
pub trait HealthIndicator {
    /// Provide the indicator of health.
    /// if include_details is true, the details will be included in the Health result.
    async fn get_health(
        &self,
        include_details: bool,
    ) -> Result<Health, Box<dyn std::error::Error + Send + Sync>> {
        let health = self.health().await?;

        let health = if include_details {
            health
        } else {
            health.without_details()
        };

        Ok(health)
    }

    /// Provide the indicator of health.
    async fn health(&self) -> Result<Health, Box<dyn std::error::Error + Send + Sync>>;
}
