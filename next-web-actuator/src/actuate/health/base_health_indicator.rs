use std::ops::Deref;

use std::error::Error as StdError;

use async_trait::async_trait;

use crate::actuate::health::health_error::HealthError;
use crate::actuate::health::{Health, HealthBuilder, health_indicator::HealthIndicator};

const DEFAULT_MESSAGE: &'static str = "Health check failed";

/// BaseHealthIndicator implementations that encapsulates creation of Health instance and
/// error handling.
pub struct BaseHealthIndicator {
    health_check_failed_message: Option<Box<dyn Fn(&dyn StdError) -> String + Send + Sync>>,
}

impl BaseHealthIndicator {
    /// Create a new BaseHealthIndicator with a custom message for health check failures.
    pub fn new<T>(health_check_failed_message: T) -> Self
    where
        T: Fn(&dyn StdError) -> String + Send + Sync,
        T: 'static,
    {
        Self {
            health_check_failed_message: Some(Box::new(health_check_failed_message)),
        }
    }

    /// Create a new BaseHealthIndicator with a default message for health check failures.
    pub fn with_message<T>(err_msg: T) -> Self
    where
        T: Into<String>,
    {
        let msg = err_msg.into();
        Self {
            health_check_failed_message: Some(Box::new(move |_error| msg.clone())),
        }
    }

    /// Handle health check failure.
    fn handle_failure(&self, error: HealthError) -> Health {
        self.log(&error);
        return HealthBuilder::new().down_with_error(error).build();
    }

    /// Log health check failure.
    fn log(&self, error: &dyn StdError) {
        let msg = self
            .health_check_failed_message
            .as_ref()
            .map(|f| f.as_ref()(error))
            .unwrap_or(DEFAULT_MESSAGE.to_string());

        tracing::warn!("{}: {}", msg, error)
    }
}

impl Default for BaseHealthIndicator {
    fn default() -> Self {
        Self {
            health_check_failed_message: None,
        }
    }
}

#[async_trait]
impl<T> HealthIndicator for T
where
    T: BaseHealthIndicatorExt,
    T: Send + Sync,
    T: Deref<Target = BaseHealthIndicator>,
{
    async fn health(&self) -> Result<Health, Box<dyn StdError + Send + Sync>> {
        let builder = HealthBuilder::new();

        match self
            .do_health_check(builder)
            .await
            .map_err(|err| self.handle_failure(HealthError::E(err.to_string())))
        {
            Ok(h) => Ok(h),
            Err(h) => Ok(h),
        }
    }
}

/// Base health indicator extension
#[async_trait]
pub trait BaseHealthIndicatorExt {
    /// Actual health check logic. If an error occurs in the pipeline, it will be handled automatically
    async fn do_health_check(
        &self,
        builder: HealthBuilder,
    ) -> Result<Health, Box<dyn StdError + Send + Sync>>;
}
