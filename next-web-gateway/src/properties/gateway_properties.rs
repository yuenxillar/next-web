use std::path::PathBuf;

use hashbrown::HashMap;
use serde_yaml::Value;

use super::{
    circuit_breaker_properties::CircuitBreakerProperties, routes_properties::RoutesProperties,
};
use crate::{
    circuit_breaker::{
        circuit_breaker_service::CircuitBreakerService,
        circuit_breaker_service_manager::CircuitBreakerServiceManager,
    },
    route::route_service_manager::RouteServiceManager,
    service::route_service::RoutePredicateService,
};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GatewayApplicationProperties {
    pub routes: Vec<RoutesProperties>,
    pub global_cors: Option<GlobalCorsProperties>,
    #[serde(skip_deserializing)]
    pub circuitbreaker: Option<Vec<CircuitBreakerProperties>>,
}

impl GatewayApplicationProperties {
    pub fn into_manager(&self) -> RouteServiceManager {
        let mut services = self
            .routes
            .iter()
            .map(|a| RoutePredicateService::from(a.clone()))
            .collect::<Vec<RoutePredicateService>>();

        // Check for duplicate service ids
        let mut seen_ids = std::collections::HashSet::new();

        for service in services.iter() {
            if !seen_ids.insert(service.id.clone()) {
                panic!("Duplicate service id found: {}", service.id);
            }
        }

        // Order services
        services.sort_by(|a, b| a.order.cmp(&b.order));

        drop(seen_ids);

        // Print all services
        services
            .iter()
            .for_each(|s| println!("Route service id: {}", s.id));

        RouteServiceManager::new(services)
    }

    pub fn into_circuitbreaker_services(&self) -> Option<CircuitBreakerServiceManager> {
        if self.circuitbreaker.is_none() {
            return None;
        }
        let circuit_breaker_properties = self.circuitbreaker.clone().unwrap();

        let mut circuit_breaker_services = HashMap::new();
        for sevice in circuit_breaker_properties
            .iter()
            .map(|a| Into::<CircuitBreakerService>::into(a.clone()))
            .collect::<Vec<CircuitBreakerService>>()
        {
            circuit_breaker_services.insert(sevice.id.clone(), sevice);
        }

        let circuit_breaker_service_manager = CircuitBreakerServiceManager {
            services: circuit_breaker_services,
        };
        Some(circuit_breaker_service_manager)
    }
}

impl Default for GatewayApplicationProperties {
    fn default() -> Self {
        let config_file = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("application.yaml")
            .display()
            .to_string();
        let data = std::fs::read_to_string(config_file).unwrap();
        let yaml_value = serde_yaml::from_str::<Value>(&data).unwrap();
        let gateway = yaml_value.get("gateway").unwrap();

        let mut gateway_properties: Self = serde_yaml::from_value(gateway.clone()).unwrap();

        if let Some(circuit_breakers) = gateway.get("circuitbreaker") {
            if let Some(mapping) = circuit_breakers.as_mapping() {
                let mut circuitbreaker = Vec::new();

                for (key, value) in mapping.iter() {
                    let id = key.as_str().unwrap().to_string();
                    let mut circuit_breaker_properties =
                        serde_yaml::from_value::<CircuitBreakerProperties>(value.clone()).unwrap();
                    circuit_breaker_properties.id = id;
                    circuitbreaker.push(circuit_breaker_properties);
                }
                gateway_properties.circuitbreaker = Some(circuitbreaker);
            }
        };

        gateway_properties
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct GlobalCorsProperties {}
