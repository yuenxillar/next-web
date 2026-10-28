//! The admin listener: health, metrics and route introspection.
//!
//! It is a separate pingora service on its own port. It never touches a route
//! or a filter, so a broken route configuration cannot take the health endpoint
//! down with it.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use async_trait::async_trait;
use bytes::Bytes;
use pingora::http::ResponseHeader;
use pingora::proxy::{ProxyHttp, Session};
use serde::Serialize;

use crate::config::{GatewayConfig, ServerConfig};
use crate::handler::predicate::RouteServiceManager;
use crate::metrics::Metrics;
use crate::service::route_service::RouteWork;

/// One route as shown by `GET /routes`.
#[derive(Debug, Clone, Serialize)]
pub struct RouteSnapshot {
    /// Route identifier.
    pub id: String,
    /// Configured `uri`.
    pub uri: String,
    /// `http`, `https` or `lb`.
    pub scheme: String,
    /// Static backends of an `lb://` route.
    pub upstreams: Vec<String>,
    /// Predicate expressions as written.
    pub predicates: Vec<String>,
    /// Filter expressions as written.
    pub filters: Vec<String>,
}

/// What the admin listener answers for one path.
pub struct AdminResponse {
    /// Status code to send.
    pub status: u16,
    /// `Content-Type` of the body.
    pub content_type: &'static str,
    /// Response body.
    pub body: String,
}

/// The admin HTTP service.
pub struct AdminService {
    config: Arc<GatewayConfig>,
    routes: Arc<RouteServiceManager>,
    metrics: Arc<Metrics>,
    started: Instant,
    ready: Arc<AtomicBool>,
}

impl AdminService {
    /// Build the service.
    ///
    /// `ready` is the proxy's own readiness flag: the admin listener reports what
    /// the proxy can actually serve, not a constant.
    pub fn new(
        config: Arc<GatewayConfig>,
        routes: Arc<RouteServiceManager>,
        metrics: Arc<Metrics>,
        ready: Arc<AtomicBool>,
    ) -> Self {
        Self {
            config,
            routes,
            metrics,
            started: Instant::now(),
            ready,
        }
    }

    /// Mark the gateway ready or not ready.
    pub fn set_ready(&self, ready: bool) {
        self.ready.store(ready, Ordering::Relaxed);
    }

    /// Whether the gateway reports itself ready.
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    /// Port and address of the admin listener, when enabled.
    pub fn listen_address(server: &ServerConfig) -> Option<&str> {
        server.admin.enabled.then_some(server.admin.bind.as_str())
    }

    /// Snapshot the configured routes.
    pub fn routes_snapshot(&self) -> Vec<RouteSnapshot> {
        self.routes
            .ordered_route_ids()
            .iter()
            .filter_map(|id| {
                let service = self.routes.services().get(id)?;
                let config = self.config.routes.iter().find(|route| &route.id == id)?;

                let scheme = match service.work {
                    RouteWork::Http => "http",
                    RouteWork::Https => "https",
                    RouteWork::LB => "lb",
                };

                Some(RouteSnapshot {
                    id: config.id.clone(),
                    uri: config.uri.clone(),
                    scheme: scheme.to_owned(),
                    upstreams: config
                        .upstreams
                        .iter()
                        .map(|upstream| upstream.addr.clone())
                        .collect(),
                    predicates: config.predicates.clone(),
                    filters: config.filters.clone(),
                })
            })
            .collect()
    }

    /// Answer one request path.
    pub fn respond(&self, path: &str) -> AdminResponse {
        let json = |status: u16, body: String| AdminResponse {
            status,
            content_type: "application/json",
            body,
        };

        match path {
            "/health/live" => json(200, "{\"status\":\"live\"}".to_owned()),
            "/health/ready" => {
                let ready = self.is_ready();
                let status = if ready { 200 } else { 503 };
                let state = if ready { "ready" } else { "not_ready" };
                json(status, format!("{{\"status\":\"{state}\"}}"))
            }
            "/metrics" => AdminResponse {
                status: 200,
                content_type: "text/plain; version=0.0.4; charset=utf-8",
                body: self.metrics.render_prometheus(),
            },
            "/routes" => match serde_json::to_string(&self.routes_snapshot()) {
                Ok(body) => json(200, body),
                Err(error) => json(500, format!("{{\"error\":\"{error}\"}}")),
            },
            "/version" => json(
                200,
                format!(
                    "{{\"name\":\"next-web-gateway\",\"version\":\"{}\",\"uptimeSeconds\":{}}}",
                    env!("CARGO_PKG_VERSION"),
                    self.started.elapsed().as_secs()
                ),
            ),
            _ => json(404, "{\"error\":\"Not Found\"}".to_owned()),
        }
    }
}

#[async_trait]
impl ProxyHttp for AdminService {
    type CTX = ();

    fn new_ctx(&self) -> Self::CTX {}

    async fn request_filter(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<bool> {
        let path = session.req_header().uri.path().to_owned();
        let response = self.respond(&path);

        let mut header = ResponseHeader::build(response.status, None)?;
        header
            .insert_header("Content-Type", response.content_type)
            .ok();
        header
            .insert_header("Content-Length", response.body.len().to_string())
            .ok();

        session
            .write_response_header(Box::new(header), false)
            .await?;
        session
            .write_response_body(Some(Bytes::from(response.body)), true)
            .await?;

        Ok(true)
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<pingora::upstreams::peer::HttpPeer>> {
        // `request_filter` answers every admin request, so this is unreachable.
        Err(crate::error::GatewayError::ServerNoRoute.into())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    use crate::config::GatewayConfig;
    use crate::handler::predicate::RouteServiceManager;
    use crate::metrics::Metrics;
    use crate::service::route_service::RoutePredicateService;

    use super::AdminService;

    fn service() -> AdminService {
        let yaml = "\
next:
  gateway:
    routes:
      - id: users
        uri: lb://user-service
        predicates: ['Path=/api/users/**']
        filters: ['StripPrefix=1']
        upstreams:
          - addr: '10.0.0.1:8080'
";
        let config = GatewayConfig::from_str(yaml).unwrap();
        let routes = RouteServiceManager::new(vec![
            RoutePredicateService::try_from(config.routes[0].clone()).unwrap(),
        ]);

        AdminService::new(
            Arc::new(config),
            Arc::new(routes),
            Arc::new(Metrics::default()),
            Arc::new(AtomicBool::new(true)),
        )
    }

    #[test]
    fn health_live_is_always_200() {
        let response = service().respond("/health/live");
        assert_eq!(response.status, 200);
        assert!(response.body.contains("live"));
    }

    #[test]
    fn routes_endpoint_lists_the_configuration() {
        let response = service().respond("/routes");
        assert_eq!(response.status, 200);
        assert!(
            response.body.contains("\"id\":\"users\""),
            "{}",
            response.body
        );
        assert!(response.body.contains("10.0.0.1:8080"), "{}", response.body);
        assert!(
            response.body.contains("Path=/api/users/**"),
            "{}",
            response.body
        );
    }

    #[test]
    fn metrics_endpoint_returns_prometheus_text() {
        let response = service().respond("/metrics");
        assert_eq!(response.status, 200);
        assert!(
            response
                .body
                .contains("# TYPE gateway_requests_total counter")
        );
    }

    #[test]
    fn admin_rejects_unknown_paths_with_404() {
        let response = service().respond("/admin/secret");
        assert_eq!(response.status, 404);
    }

    #[test]
    fn health_ready_follows_the_shared_readiness_flag() {
        let service = service();
        assert_eq!(service.respond("/health/ready").status, 200);

        // The proxy flips the same flag when a pool cannot produce a backend.
        service.set_ready(false);
        assert_eq!(
            service.respond("/health/ready").status,
            503,
            "readiness must follow the proxy, not a constant"
        );

        service.set_ready(true);
        assert_eq!(service.respond("/health/ready").status, 200);
    }

    #[test]
    fn version_names_the_crate() {
        let response = service().respond("/version");
        assert!(response.body.contains("next-web-gateway"));
    }
}
