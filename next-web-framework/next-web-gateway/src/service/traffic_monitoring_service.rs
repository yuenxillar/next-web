use std::time::Duration;

use async_trait::async_trait;
use pingora::{server::ShutdownWatch, services::background::BackgroundService};
use tokio::time::interval;

#[derive(Clone)]
pub struct TrafficMonitoringService {}

impl TrafficMonitoringService {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl BackgroundService for TrafficMonitoringService {
    async fn start(&self, mut shutdown: ShutdownWatch) {
        let mut period = interval(Duration::from_secs(10));
        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    // shutdown
                    break;
                }
                _ = period.tick() => {
                    // do some work
                    // info!("doing some work")
                }
            }
        }
    }
}
