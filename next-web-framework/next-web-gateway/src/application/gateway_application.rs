use super::next_gateway_application::NextGatewayApplication;
use crate::circuit_breaker::fallback_provider::FallbackProvider;
use crate::{
    background_service::traffic_monitoring_service::TrafficMonitoringService,
    properties::gateway_properties::GatewayApplicationProperties,
};
use async_trait::async_trait;
use pingora::prelude::background_service;
use pingora::services::Service;
use pingora::{prelude::Opt, proxy::http_proxy_service, server::Server};

#[cfg(unix)]
#[cfg(feature = "jemalloc")]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[async_trait]
pub trait GatewayApplication: Send + Sync {
    // Get
    fn fallback_providers(&self) -> Vec<Box<dyn FallbackProvider>> {
        vec![]
    }

    fn init_logging(&self) {
        let config = tracing_subscriber::fmt::format()
            .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
                "%Y-%m-%d %H:%M:%S%.3f".to_string(),
            ))
            .with_level(true)
            .with_target(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_thread_names(true);

        // tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_ansi(false)
            .event_format(config)
            .init();
    }

    async fn run()
    where
        Self: GatewayApplication + Default,
    {
        let user_application = Self::default();

        user_application.init_logging();

        // Retrieve configuration files and convert them into objects
        let application_properties = GatewayApplicationProperties::new();
        let route_service_manager = application_properties.into_services();
        let mut circuitbreaker_service_manager =
            application_properties.into_circuitbreaker_services();

        let fallback_providers = user_application.fallback_providers();

        if let Some(manager) = circuitbreaker_service_manager.as_mut() {
            manager.set_fallback_providers(fallback_providers);
        }

        let gateway_application = NextGatewayApplication::new(
            application_properties,
            route_service_manager,
            circuitbreaker_service_manager,
        );

        // Create backgroud services
        let traffic_monitoring_service =
            background_service("TrafficMonitoringService", TrafficMonitoringService::new());

        // Create a gateway server
        let mut gateway_server = Server::new(Opt::default()).unwrap();

        gateway_server.bootstrap();

        let mut proxy_service =
            http_proxy_service(&gateway_server.configuration, gateway_application);
        proxy_service.add_tcp("127.0.0.1:8080");

        let services: Vec<Box<dyn Service>> = vec![
            Box::new(proxy_service),
            Box::new(traffic_monitoring_service),
        ];
        gateway_server.add_services(services);
        gateway_server.run_forever();
    }
}
