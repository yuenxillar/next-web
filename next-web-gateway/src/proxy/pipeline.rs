//! The gateway request lifecycle.
//!
//! Order of operations for one request:
//!
//! 1. `request_filter` assigns the request id, matches a route, runs the
//!    `RequestHeader` filters, enforces the body limits and reserves a circuit
//!    breaker probe. Anything rejected here answers without touching an
//!    upstream.
//! 2. `upstream_peer` resolves the backend and builds the peer.
//! 3. `upstream_request_filter` adds the forwarding headers and strips the
//!    hop-by-hop ones.
//! 4. `response_filter` records the breaker outcome, runs the `ResponseHeader`
//!    filters and decides whether the body must be buffered.
//! 5. `response_body_filter` applies the body filters and stores the cache
//!    entry.
//! 6. `logging` writes one access log line and updates the metrics.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use bytes::Bytes;
use pingora::http::{RequestHeader, ResponseHeader};
use pingora::proxy::{FailToProxy, ProxyHttp, Session};
use pingora::upstreams::peer::HttpPeer;
use tracing::{error, info, warn};

use crate::circuit_breaker::breaker_controller::Outcome;
use crate::circuit_breaker::circuit_breaker_service_manager::CircuitBreakerServiceManager;
use crate::config::{ErrorConfig, GatewayConfig, RouteConfig};
use crate::context::{HeaderAndBody, RequestContext};
use crate::error::GatewayError;
use crate::filter::factory::local_response_cache::{
    capture_response_for_local_cache, serve_from_local_cache_if_present, store_local_cache_entry,
    LocalResponseCacheManager,
};
use crate::filter::spec::FilterPhase;
use crate::handler::predicate::RouteServiceManager;
use crate::metrics::Metrics;
use crate::proxy::error_response::{render_error, status_for_error};
use crate::service::route_service::RouteWork;
use crate::upstream::{build_peer, DnsCache, StaticUpstreams, UpstreamResolver, UpstreamSelection};

/// Maximum response body buffered for a body filter, in bytes.
const MAX_BUFFER_BYTES: usize = 8 * 1024 * 1024;

/// Request headers never forwarded to the upstream.
const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "upgrade",
    "proxy-connection",
];

/// The gateway itself.
pub struct GatewayProxy {
    config: Arc<GatewayConfig>,
    routes: Arc<RouteServiceManager>,
    upstreams: Arc<StaticUpstreams>,
    dns: DnsCache,
    breakers: Option<Arc<CircuitBreakerServiceManager>>,
    cache: Option<Arc<LocalResponseCacheManager>>,
    metrics: Arc<Metrics>,
}

impl GatewayProxy {
    /// Assemble the proxy from a validated configuration.
    pub fn new(config: GatewayConfig) -> Result<Self, crate::config::ConfigErrors> {
        let mut route_services = Vec::with_capacity(config.routes.len());
        let mut errors = Vec::new();

        for route in &config.routes {
            match crate::service::route_service::RoutePredicateService::try_from(route.clone()) {
                Ok(service) => route_services.push(service),
                Err(error) => errors.extend(error.errors().iter().cloned()),
            }
        }

        if !errors.is_empty() {
            return Err(crate::config::ConfigErrors::new(errors));
        }

        // A `CircuitBreaker=<id>` that names an undefined breaker is a typo, not
        // a silent no-op.
        let mut errors = Vec::new();
        for route in &route_services {
            if !route.fallback_id.is_empty() && !config.breakers.contains_key(&route.fallback_id) {
                errors.push(crate::config::ConfigError::new(
                    format!("routes.{}.filters", route.id),
                    format!(
                        "unknown circuit breaker `{}`, defined breakers: {}",
                        route.fallback_id,
                        if config.breakers.is_empty() {
                            "none".to_string()
                        } else {
                            config
                                .breakers
                                .keys()
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ")
                        }
                    ),
                ));
            }
        }

        if !errors.is_empty() {
            return Err(crate::config::ConfigErrors::new(errors));
        }

        let upstreams = StaticUpstreams::build(&config)?;
        let breakers = CircuitBreakerServiceManager::from_config(&config.breakers);
        let cache = config
            .local_response_cache_enabled()
            .then(LocalResponseCacheManager::shared);

        Ok(Self {
            config: Arc::new(config),
            routes: Arc::new(RouteServiceManager::new(route_services)),
            upstreams: Arc::new(upstreams),
            dns: DnsCache::default(),
            breakers: Some(Arc::new(breakers)),
            cache,
            metrics: Arc::new(Metrics::default()),
        })
    }

    /// Routes, for the admin listener.
    pub fn routes(&self) -> &RouteServiceManager {
        &self.routes
    }

    /// Shared route table, for the admin listener.
    pub fn shared_routes(&self) -> Arc<RouteServiceManager> {
        self.routes.clone()
    }

    /// Metrics, for the admin listener.
    pub fn metrics(&self) -> &Arc<Metrics> {
        &self.metrics
    }

    /// Configuration, for the admin listener.
    pub fn config(&self) -> &GatewayConfig {
        &self.config
    }

    /// Whether every `lb://` route has a backend.
    pub fn is_ready(&self) -> bool {
        self.routes
            .ordered_route_ids()
            .iter()
            .filter(|route_id| {
                self.routes
                    .services()
                    .get(*route_id)
                    .map(|service| service.work == RouteWork::LB)
                    .unwrap_or(false)
            })
            .all(|route_id| self.upstreams.is_ready(route_id))
    }

    fn route_config(&self, route_id: &str) -> Option<&RouteConfig> {
        self.config.routes.iter().find(|route| route.id == route_id)
    }

    /// Write a gateway generated response and report the exchange as handled.
    async fn respond_directly(
        &self,
        session: &mut Session,
        ctx: &mut RequestContext,
        status: u16,
    ) -> pingora::Result<bool> {
        let route = ctx.route_id.as_deref();
        let rendered = render_error(&self.config.error, status, route, ctx.request_id.as_str());

        let (mut header, body) = match rendered {
            Ok(rendered) => rendered,
            Err(error) => {
                error!("failed to build the error response for {status}: {error}");
                session.respond_error(500).await?;
                return Ok(true);
            }
        };

        // Headers a filter attached when it rejected the request (Retry-After,
        // X-RateLimit-*, Location for RedirectTo) must survive the rendering.
        if let Some(direct) = ctx.direct_response.as_ref() {
            for (name, value) in &direct.headers {
                header.insert_header(name.clone(), value.as_str()).ok();
            }
        }

        // Gateway generated responses go through the response filters too, so
        // `SecureHeaders` and `AddResponseHeader` apply to errors as well.
        self.routes
            .filter_phase(
                ctx,
                FilterPhase::ResponseHeader,
                HeaderAndBody::with_resp_header(&mut header),
            )
            .await
            .ok();

        // Throttled and unavailable responses tell the client when to retry.
        if matches!(status, 429 | 503) && !header.headers.contains_key("retry-after") {
            header.insert_header("Retry-After", "1").ok();
        }

        write_response(session, header, body).await;
        Ok(true)
    }
}

/// Write a complete response to the downstream.
async fn write_response(session: &mut Session, mut header: ResponseHeader, body: Bytes) {
    header
        .insert_header("Content-Length", body.len().to_string())
        .ok();

    if let Err(error) = session.write_response_header(Box::new(header), false).await {
        error!("failed to write the response header: {error}");
        return;
    }

    if let Err(error) = session.write_response_body(Some(body), true).await {
        error!("failed to write the response body: {error}");
    }
}

/// Request id from the configured header, or a fresh UUID.
fn request_id(header: &RequestHeader, config: &ErrorConfig) -> String {
    header
        .headers
        .get(config.request_id_header.as_str())
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

/// Downstream peer address, if the connection is an IP socket.
fn client_ip(session: &Session) -> Option<std::net::IpAddr> {
    match session.client_addr()? {
        pingora::protocols::l4::socket::SocketAddr::Inet(addr) => Some(addr.ip()),
        #[cfg(unix)]
        pingora::protocols::l4::socket::SocketAddr::Unix(_) => None,
    }
}

/// Hash used as the load balancer key.
fn selection_key(ctx: &RequestContext, path: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    ctx.client_ip.hash(&mut hasher);
    path.hash(&mut hasher);
    hasher.finish()
}

#[async_trait]
impl ProxyHttp for GatewayProxy {
    type CTX = RequestContext;

    fn new_ctx(&self) -> Self::CTX {
        RequestContext {
            local_response_cache_manager: self.cache.clone(),
            ..RequestContext::default()
        }
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> pingora::Result<bool> {
        ctx.started = Some(Instant::now());
        ctx.client_ip = client_ip(session);

        // The request id must exist before anything can answer, including the
        // 404 for an unmatched route.
        let header_name = self.config.error.request_id_header.clone();
        let id = request_id(session.req_header(), &self.config.error);
        ctx.request_id = id.clone();
        session
            .req_header_mut()
            .insert_header(header_name, id)
            .ok();

        let Some(route) = self.routes.match_route(session) else {
            return self.respond_directly(session, ctx, 404).await;
        };

        ctx.route_id = Some(route.route_id.clone());
        ctx.fallback_id = (!route.fallback_id.is_empty()).then(|| route.fallback_id.clone());
        ctx.path_variables = route.path_variables.clone();

        let header = session.req_header_mut();

        // Request header filters, exactly once per request.
        if let Err(error) = self
            .routes
            .filter_phase(ctx, FilterPhase::RequestHeader, HeaderAndBody::with_req_header(header))
            .await
        {
            let status = ctx
                .direct_response
                .as_ref()
                .map(|response| response.status)
                .unwrap_or_else(|| status_for_error(&error));
            return self.respond_directly(session, ctx, status).await;
        }
        if ctx.direct_response.is_some() {
            let status = ctx.direct_response.as_ref().map(|r| r.status).unwrap_or(500);
            return self.respond_directly(session, ctx, status).await;
        }

        // Reserve a breaker probe before connecting.
        if let Some(breaker_id) = ctx.fallback_id.clone() {
            if let Some(breakers) = self.breakers.as_ref() {
                if let Some(service) = breakers.services.get(&breaker_id) {
                    if service.controller.try_acquire().await.is_err() {
                        return self.respond_directly(session, ctx, 503).await;
                    }
                }
            }
        }

        self.metrics.record_inflight(1);
        Ok(false)
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        let Some(route_id) = ctx.route_id.clone() else {
            return Err(GatewayError::ServerNoRoute.into());
        };
        let Some(route_config) = self.route_config(&route_id) else {
            return Err(GatewayError::ServerNoRoute.into());
        };
        let Some(route) = self.routes.services().get(&route_id).cloned() else {
            return Err(GatewayError::ServerNoRoute.into());
        };

        let parsed = crate::upstream::UpstreamUri::parse(&route_config.uri)
            .map_err(|_| GatewayError::ServerNoUpstreamServices)?;

        let path = session.req_header().uri.path().to_string();
        let group_key = selection_key(ctx, &path);

        let selection = match route.work {
            RouteWork::LB => {
                let key = group_key.to_be_bytes();
                let Some(addr) = self.upstreams.select(&route_id, &key) else {
                    return Err(GatewayError::ServerNoUpstreamServices.into());
                };
                UpstreamSelection {
                    addr,
                    sni: parsed.host.clone(),
                    host_header: addr.to_string(),
                    tls: false,
                    group_key,
                }
            }
            RouteWork::Http | RouteWork::Https => {
                let tls = route.work == RouteWork::Https;
                let addresses = self
                    .dns
                    .resolve(&parsed.host, parsed.port)
                    .await
                    .map_err(|error| {
                        warn!(route = %route_id, "upstream name resolution failed: {error}");
                        GatewayError::ServerNoUpstreamServices
                    })?;

                let Some(addr) = addresses.first().copied() else {
                    return Err(GatewayError::ServerNoUpstreamServices.into());
                };

                UpstreamSelection {
                    addr,
                    sni: parsed.host.clone(),
                    host_header: if parsed.port == 80 || parsed.port == 443 {
                        parsed.host.clone()
                    } else {
                        parsed.authority()
                    },
                    tls,
                    group_key,
                }
            }
        };

        ctx.selected = Some(selection.clone());
        let peer = build_peer(&selection, &self.config.upstream, route_config);
        Ok(Box::new(peer))
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        ctx.original_request_path = upstream_request
            .uri
            .path_and_query()
            .map(|value| value.as_str().to_string());

        for name in HOP_BY_HOP {
            upstream_request.remove_header(*name);
        }

        if let Some(cache) = self
            .routes
            .local_response_cache_filter(ctx.route_id.as_deref())
        {
            serve_from_local_cache_if_present(cache, ctx, upstream_request)?;
        }

        // Forwarding headers: appended, never overwritten.
        let peer_ip = ctx
            .client_ip
            .map(|ip| ip.to_string())
            .unwrap_or_default();
        if !peer_ip.is_empty() {
            match upstream_request
                .headers
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok())
                .map(str::to_string)
            {
                Some(existing) if !existing.is_empty() => {
                    upstream_request
                        .insert_header("X-Forwarded-For", format!("{existing}, {peer_ip}"))
                        .ok();
                }
                _ => {
                    upstream_request.insert_header("X-Forwarded-For", peer_ip).ok();
                }
            }
        }

        upstream_request
            .insert_header("X-Forwarded-Proto", "http")
            .ok();
        if let Some(host) = upstream_request
            .headers
            .get("host")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
        {
            upstream_request.insert_header("X-Forwarded-Host", host).ok();
        }

        // Host default: the upstream authority, unless the route preserves it.
        if ctx.original_host.is_none() {
            if let Some(selection) = ctx.selected.as_ref() {
                upstream_request
                    .insert_header("Host", selection.host_header.clone())
                    .ok();
            }
        }

        Ok(())
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        if let Some(breaker_id) = ctx.fallback_id.clone() {
            if let Some(breakers) = self.breakers.as_ref() {
                if let Some(service) = breakers.services.get(&breaker_id) {
                    let outcome = if upstream_response.status.as_u16() >= 500 {
                        Outcome::Failure
                    } else {
                        Outcome::Success
                    };
                    service.controller.process(outcome).await;
                }
            }
        }

        self.routes
            .filter_phase(
                ctx,
                FilterPhase::ResponseHeader,
                HeaderAndBody::with_resp_header(upstream_response),
            )
            .await?;

        if let Some(cache) = self
            .routes
            .local_response_cache_filter(ctx.route_id.as_deref())
        {
            capture_response_for_local_cache(cache, ctx, upstream_response);
        }

        let must_buffer = self
            .routes
            .has_response_body_filters(ctx.route_id.as_deref())
            || ctx.pending_local_response_cache.is_some();

        if must_buffer {
            ctx.buffer_response_body = true;
            ctx.response_body_buffer.clear();
            ctx.buffer_overflowed = false;
            upstream_response.remove_header("Content-Length");
            upstream_response.remove_header("Transfer-Encoding");
            upstream_response
                .insert_header("Transfer-Encoding", "Chunked")
                .ok();
        }

        Ok(())
    }

    async fn request_body_filter(
        &self,
        _session: &mut Session,
        body: &mut Option<Bytes>,
        _end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        if let Some(chunk) = body.as_ref() {
            ctx.request_body_bytes = ctx.request_body_bytes.saturating_add(chunk.len() as u64);
        }

        // A chunked request carries no `Content-Length`, so the limit can only
        // be enforced while the body streams through.
        let Some(limit) = self.routes.request_size_limit(ctx.route_id.as_deref()) else {
            return Ok(());
        };

        if ctx.request_body_bytes > limit {
            warn!(
                route = ctx.route_id.as_deref().unwrap_or("-"),
                limit, seen = ctx.request_body_bytes, "request body exceeds the configured limit"
            );
            return ctx.respond_with_text(413, Vec::new(), "Request size exceeded");
        }

        Ok(())
    }

    fn response_body_filter(
        &self,
        _session: &mut Session,
        body: &mut Option<Bytes>,
        end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<Option<Duration>> {        let Some(chunk) = body.take() else {
            return Ok(None);
        };

        if !ctx.buffer_response_body {
            *body = Some(chunk);
            return Ok(None);
        }

        if ctx.response_body_buffer.len() + chunk.len() > MAX_BUFFER_BYTES {
            // Give up on rewriting rather than buffering without a bound.
            warn!(
                route = ctx.route_id.as_deref().unwrap_or("-"),
                "response body exceeds the {MAX_BUFFER_BYTES} byte rewrite limit, streaming it"
            );
            ctx.buffer_overflowed = true;
            ctx.buffer_response_body = false;
            let mut flushed = Bytes::from(std::mem::take(&mut ctx.response_body_buffer));
            let mut combined = Vec::with_capacity(flushed.len() + chunk.len());
            combined.extend_from_slice(&flushed);
            combined.extend_from_slice(&chunk);
            flushed = Bytes::from(combined);
            *body = Some(flushed);
            return Ok(None);
        }

        ctx.response_body_buffer.extend_from_slice(&chunk);

        if !end_of_stream {
            return Ok(None);
        }

        let mut buffered = Bytes::from(std::mem::take(&mut ctx.response_body_buffer));
        self.routes.apply_body_filters(ctx, &mut buffered);
        store_local_cache_entry(ctx, Some(&buffered));
        ctx.buffer_response_body = false;
        *body = Some(buffered);

        Ok(None)
    }

    fn suppress_error_log(&self, _session: &Session, _ctx: &Self::CTX, _error: &pingora::Error) -> bool {
        false
    }

    fn error_while_proxy(
        &self,
        peer: &HttpPeer,
        session: &mut Session,
        e: Box<pingora::Error>,
        ctx: &mut Self::CTX,
        client_reused: bool,
    ) -> Box<pingora::Error> {
        if let Some(breaker_id) = ctx.fallback_id.clone() {
            if let Some(breakers) = self.breakers.clone() {
                tokio::spawn(async move {
                    if let Some(service) = breakers.services.get(&breaker_id) {
                        service.controller.process(Outcome::Failure).await;
                    }
                });
            }
        }

        let mut e = e.more_context(format!("Peer: {peer}"));
        e.retry
            .decide_reuse(client_reused && !session.as_ref().retry_buffer_truncated());
        e
    }

    fn fail_to_connect(
        &self,
        _session: &mut Session,
        _peer: &HttpPeer,
        ctx: &mut Self::CTX,
        e: Box<pingora::Error>,
    ) -> Box<pingora::Error> {
        if let Some(breaker_id) = ctx.fallback_id.clone() {
            if let Some(breakers) = self.breakers.clone() {
                tokio::spawn(async move {
                    if let Some(service) = breakers.services.get(&breaker_id) {
                        service.controller.process(Outcome::Failure).await;
                    }
                });
            }
        }

        e
    }

    async fn fail_to_proxy(
        &self,
        session: &mut Session,
        e: &pingora::Error,
        ctx: &mut Self::CTX,
    ) -> FailToProxy {
        self.metrics
            .observe_request(
                ctx.route_id.as_deref().unwrap_or("-"),
                "-",
                status_for_error(e),
                Duration::from_millis(ctx.elapsed_millis() as u64),
            );

        let status = match ctx.direct_response.as_ref() {
            Some(response) => response.status,
            None => status_for_error(e),
        };

        if status == 0 {
            return FailToProxy {
                error_code: 0,
                can_reuse_downstream: false,
            };
        }

        let handled = self.respond_directly(session, ctx, status).await;
        if handled.is_err() {
            error!("failed to write the error response");
        }

        FailToProxy {
            error_code: status,
            can_reuse_downstream: false,
        }
    }

    async fn logging(
        &self,
        session: &mut Session,
        error: Option<&pingora::Error>,
        ctx: &mut Self::CTX,
    ) {
        let status = session
            .response_written()
            .map(|response| response.status.as_u16())
            .unwrap_or(0);
        let latency = Duration::from_millis(ctx.elapsed_millis() as u64);
        let route = ctx.route_id.as_deref().unwrap_or("-");
        let upstream = ctx
            .selected
            .as_ref()
            .map(|selection| selection.addr.to_string())
            .unwrap_or_else(|| "-".to_string());

        self.metrics
            .observe_request(route, session.req_header().method.as_str(), status, latency);
        self.metrics.record_inflight(-1);

        if self.config.logging.access_log {
            match error {
                Some(error) => info!(
                    request_id = %ctx.request_id,
                    client = ?ctx.client_ip,
                    method = %session.req_header().method,
                    path = %session.req_header().uri.path(),
                    status,
                    route = %route,
                    upstream = %upstream,
                    latency_ms = latency.as_millis(),
                    error = %error,
                    "access"
                ),
                None => info!(
                    request_id = %ctx.request_id,
                    client = ?ctx.client_ip,
                    method = %session.req_header().method,
                    path = %session.req_header().uri.path(),
                    status,
                    route = %route,
                    upstream = %upstream,
                    latency_ms = latency.as_millis(),
                    "access"
                ),
            }
        }
    }
}
