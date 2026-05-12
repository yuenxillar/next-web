use std::path::{Path, PathBuf};

use hashbrown::HashMap;
use serde_yaml::Value;
use tracing::info;

use super::{
    circuit_breaker_properties::CircuitBreakerProperties, routes_properties::RoutesProperties,
};
use crate::{
    circuit_breaker::{
        circuit_breaker_service::CircuitBreakerService,
        circuit_breaker_service_manager::CircuitBreakerServiceManager,
    },
    handler::predicate::RouteServiceManager,
    service::route_service::RoutePredicateService,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GatewayApplicationProperties {
    pub routes: Vec<RoutesProperties>,
    pub global_cors: Option<GlobalCorsProperties>,
    pub local_response_cache: Option<LocalResponseCacheProperties>,
    #[serde(skip_deserializing)]
    pub circuitbreaker: Option<Vec<CircuitBreakerProperties>>,
}

impl GatewayApplicationProperties {
    pub const CONFIG_ENV_VAR: &'static str = "NEXT_WEB_GATEWAY_CONFIG";

    pub fn into_manager(&self) -> RouteServiceManager {
        let mut services = self
            .routes
            .iter()
            .map(|route| RoutePredicateService::from(route.clone()))
            .collect::<Vec<RoutePredicateService>>();

        let mut seen_ids = std::collections::HashSet::new();
        for service in services.iter() {
            if !seen_ids.insert(service.id.clone()) {
                panic!("Duplicate service id found: {}", service.id);
            }
        }

        services.sort_by(|a, b| a.order.cmp(&b.order));
        services
            .iter()
            .for_each(|service| info!("route service id: {}", service.id));

        RouteServiceManager::new(services)
    }

    pub fn into_circuitbreaker_services(&self) -> Option<CircuitBreakerServiceManager> {
        let circuit_breaker_properties = self.circuitbreaker.clone()?;

        let mut circuit_breaker_services = HashMap::new();
        for service in circuit_breaker_properties
            .iter()
            .map(|properties| Into::<CircuitBreakerService>::into(properties.clone()))
            .collect::<Vec<CircuitBreakerService>>()
        {
            circuit_breaker_services.insert(service.id.clone(), service);
        }

        Some(CircuitBreakerServiceManager {
            services: circuit_breaker_services,
        })
    }

    pub async fn load() -> Result<Self> {
        Self::load_from_env_or_default().await
    }

    pub async fn load_from_env_or_default() -> Result<Self> {
        if let Ok(path) = std::env::var(Self::CONFIG_ENV_VAR) {
            return Self::load_from_path(path).await;
        }

        Self::load_from_path("application.yaml").await
    }

    pub async fn load_from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let data = tokio::fs::read_to_string(path).await.map_err(|error| {
            std::io::Error::other(format!(
                "failed to read gateway config '{}': {}",
                path.display(),
                error
            ))
        })?;

        Self::from_yaml_str(&data)
    }

    pub fn from_yaml_str(data: &str) -> Result<Self> {
        let yaml_value = serde_yaml::from_str::<Value>(data).map_err(|error| {
            std::io::Error::other(format!("failed to parse gateway yaml: {error}"))
        })?;
        let gateway = yaml_value
            .get("gateway")
            .ok_or_else(|| std::io::Error::other("missing top-level 'gateway' section"))?;

        let mut gateway_properties =
            serde_yaml::from_value::<Self>(gateway.clone()).map_err(|error| {
                std::io::Error::other(format!("failed to deserialize gateway properties: {error}"))
            })?;

        if let Some(circuit_breakers) = gateway.get("circuitbreaker") {
            let mapping = circuit_breakers.as_mapping().ok_or_else(|| {
                std::io::Error::other("'gateway.circuitbreaker' must be a mapping")
            })?;
            let mut circuitbreaker = Vec::new();

            for (key, value) in mapping.iter() {
                let id = key
                    .as_str()
                    .ok_or_else(|| std::io::Error::other("circuit breaker id must be a string"))?
                    .to_string();
                let mut properties = serde_yaml::from_value::<CircuitBreakerProperties>(
                    value.clone(),
                )
                .map_err(|error| {
                    std::io::Error::other(format!(
                        "failed to deserialize circuit breaker '{id}': {error}"
                    ))
                })?;
                properties.id = id;
                circuitbreaker.push(properties);
            }

            gateway_properties.circuitbreaker = Some(circuitbreaker);
        }

        Ok(gateway_properties)
    }

    pub fn local_response_cache_enabled(&self) -> bool {
        self.local_response_cache
            .as_ref()
            .and_then(|config| config.enabled)
            .unwrap_or(false)
    }
}

impl Default for GatewayApplicationProperties {
    fn default() -> Self {
        let path = std::env::var(Self::CONFIG_ENV_VAR)
            .unwrap_or_else(|_| PathBuf::from("application.yaml").display().to_string());

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap_or_else(|error| {
                panic!("failed to build tokio runtime for gateway config: {error}")
            });

        runtime
            .block_on(Self::load_from_path(path))
            .unwrap_or_else(|error| panic!("failed to load gateway config: {error}"))
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GlobalCorsProperties {}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LocalResponseCacheProperties {
    pub enabled: Option<bool>,
}
