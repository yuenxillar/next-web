//! Heartbeat mechanism for SockJS sessions.
//!
//! SockJS requires periodic heartbeat frames to keep connections alive
//! and prevent proxies from closing idle connections.

use std::{sync::Arc, time::Duration};

use tokio::{task::JoinHandle, time};
use tracing::{debug, warn};

use crate::sockjs::session::SockJsSession;

/// Handle for a running heartbeat task.
///
/// Manages a background task that sends periodic heartbeat frames
/// to a SockJS session.
pub struct HeartbeatHandle {
    task: JoinHandle<()>,
}

impl HeartbeatHandle {
    /// Starts a new heartbeat task for the given session.
    ///
    /// # Arguments
    ///
    /// * `session` - The SockJS session to send heartbeats to
    /// * `interval_ms` - Heartbeat interval in milliseconds
    ///
    /// # Returns
    ///
    /// A handle to the running heartbeat task.
    pub fn spawn(session: Arc<SockJsSession>, interval_ms: u64) -> Self {
        let mut interval = time::interval(Duration::from_millis(interval_ms.max(1)));
        interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        let task = tokio::spawn(async move {
            // `interval` ticks immediately on its first call. Skip that tick
            // so the first heartbeat is sent only after the configured delay.
            interval.tick().await;
            loop {
                interval.tick().await;

                // Check if session is still alive
                if session.is_closed().await {
                    debug!("Session closed, stopping heartbeat: {}", session.id());
                    break;
                }

                // Send heartbeat
                if let Err(e) = session.send_heartbeat().await {
                    warn!("Failed to send heartbeat to {}: {}", session.id(), e);
                    break;
                }

                debug!("Heartbeat sent to session: {}", session.id());
            }
        });

        Self { task }
    }

    /// Stops the heartbeat task.
    pub fn stop(self) {
        self.task.abort();
    }

    /// Checks if the heartbeat task is still running.
    pub fn is_running(&self) -> bool {
        !self.task.is_finished()
    }
}

impl Drop for HeartbeatHandle {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sockjs::{codec::Jackson2SockJsMessageCodec, session::SockJsSession};
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_heartbeat_task() {
        let session = Arc::new(SockJsSession::new(
            "test-session".to_string(),
            Arc::new(Jackson2SockJsMessageCodec),
            "127.0.0.1:8080".parse::<SocketAddr>().unwrap(),
            axum::http::HeaderMap::new(),
        ));

        // Start heartbeat with 100ms interval
        let handle = HeartbeatHandle::spawn(session.clone(), 100);

        // Wait a bit
        tokio::time::sleep(Duration::from_millis(350)).await;

        // Should have received some heartbeats
        assert!(handle.is_running());

        // Stop heartbeat
        handle.stop();
    }
}
