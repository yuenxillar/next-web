use std::{io::ErrorKind, time::Duration};

use next_web_core::async_trait;
use rumqttc::ConnectionError;
use tracing::error;

/// Context passed to the MQTT poll error handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MQTTPollErrorContext {
    /// Configured maximum retries.
    pub max_retries: usize,
    /// Remaining retries after the current error has been accounted for.
    pub retries_remaining: usize,
}

/// Action returned by the MQTT poll error handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MQTTPollErrorAction {
    /// Continue polling after the given delay.
    RetryAfter(Duration),
    /// Stop the event loop and request graceful shutdown.
    Shutdown,
}

/// Strategy hook used to handle `eventloop.poll()` failures.
///
/// Applications can bind an `Arc<dyn MQTTPollErrorHandler>` to override the
/// default retry and shutdown behavior.
#[async_trait]
pub trait MQTTPollErrorHandler
where
    Self: Send + Sync,
    Self: 'static,
{
    /// Handle a poll error and decide whether the event loop should retry or
    /// stop the application.
    async fn handle_poll_error(
        &self,
        error: &ConnectionError,
        context: MQTTPollErrorContext,
    ) -> MQTTPollErrorAction;
}

/// Default MQTT poll error handler preserving the previous behavior.
#[derive(Debug, Clone)]
pub struct DefaultMQTTPollErrorHandler {
    retry_delay: Duration,
}

impl Default for DefaultMQTTPollErrorHandler {
    fn default() -> Self {
        Self {
            retry_delay: Duration::from_millis(300),
        }
    }
}

impl DefaultMQTTPollErrorHandler {
    /// Create a handler with an explicit retry delay.
    pub fn new(retry_delay: Duration) -> Self {
        Self { retry_delay }
    }
}

#[async_trait]
impl MQTTPollErrorHandler for DefaultMQTTPollErrorHandler {
    async fn handle_poll_error(
        &self,
        error: &ConnectionError,
        context: MQTTPollErrorContext,
    ) -> MQTTPollErrorAction {
        if is_connection_refused(error) && context.retries_remaining == 0 {
            error!(
                "MQTT connection refused after {} retries: {:?}",
                context.max_retries, error
            );
            return MQTTPollErrorAction::Shutdown;
        }

        error!("MQTT eventloop error: {:?}", error);
        MQTTPollErrorAction::RetryAfter(self.retry_delay)
    }
}

/// Returns `true` when the poll error represents a connection refusal.
pub(crate) fn is_connection_refused(error: &ConnectionError) -> bool {
    match error {
        ConnectionError::Io(io_error) => io_error.kind() == ErrorKind::ConnectionRefused,
        ConnectionError::ConnectionRefused(_) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_handler_requests_shutdown_after_last_retry() {
        let handler = DefaultMQTTPollErrorHandler::default();
        let error = ConnectionError::Io(std::io::Error::new(
            ErrorKind::ConnectionRefused,
            "connection refused",
        ));

        let action = handler
            .handle_poll_error(
                &error,
                MQTTPollErrorContext {
                    max_retries: 3,
                    retries_remaining: 0,
                },
            )
            .await;

        assert_eq!(action, MQTTPollErrorAction::Shutdown);
    }

    #[tokio::test]
    async fn default_handler_retries_for_non_terminal_errors() {
        let handler = DefaultMQTTPollErrorHandler::default();
        let error = ConnectionError::NetworkTimeout;

        let action = handler
            .handle_poll_error(
                &error,
                MQTTPollErrorContext {
                    max_retries: 3,
                    retries_remaining: 2,
                },
            )
            .await;

        assert_eq!(
            action,
            MQTTPollErrorAction::RetryAfter(Duration::from_millis(300))
        );
    }
}
