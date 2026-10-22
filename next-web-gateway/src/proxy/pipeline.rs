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
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use bytes::Bytes;
use pingora::http::{RequestHeader, ResponseHeader};
use pingora::proxy::{FailToProxy, ProxyHttp, Session};
use pingora::upstreams::peer::HttpPeer;
use tracing::{error, info, warn};

use crate::circuit_breaker::breaker_controller::Outcome;
use crate::circuit_breaker::circuit_breaker_service_manager::CircuitBreakerServiceManager;
use crate::circuit_breaker::circuit_state::CircuitState;
use crate::config::{ErrorConfig, GatewayConfig, RouteConfig};
use crate::context::{HeaderAndBody, RequestContext};
use crate::error::GatewayError;
use crate::filter::factory::local_response_cache::{
    LocalResponseCacheManager, capture_response_for_local_cache, serve_from_local_cache_if_present,
    store_local_cache_entry,
};
use crate::filter::spec::FilterPhase;
use crate::handler::predicate::RouteServiceManager;
use crate::metrics::Metrics;
use crate::proxy::error_response::{render_error, status_for_error};
use crate::service::route_service::RouteWork;
use crate::upstream::{
    DnsCache, StaticUpstreams, UpstreamResolver, UpstreamSelection, build_peer, load_ca_bundle,
};

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
    ready: Arc<AtomicBool>,
    ca: Option<Arc<crate::upstream::CaBundle>>,
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

        // A trust anchor is loaded once at startup: a bundle that cannot be read
        // or parsed must fail here, not on the first TLS request.
        let ca = match load_ca_bundle(&config.upstream.tls) {
            Ok(ca) => ca,
            Err(error) => return Err(crate::config::ConfigErrors::new(vec![error])),
        };
        let mut errors = Vec::new();
        for route in &route_services {
            if !route.fallback_id.is_empty() && !config.breakers.contains_key(&route.fallback_id) {
                errors.push(crate::config::ConfigError::new(
                    format!("routes.{}.filters", route.id),
                    format!(
                        "unknown circuit breaker `{}`, defined breakers: {}",
                        route.fallback_id,
                        if config.breakers.is_empty() {
                            "none".to_owned()
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

        // Install the trusted proxy list once, before any request is served:
        // the XFF predicate and the rate limiter both read it.
        let mut trusted = Vec::with_capacity(config.trusted_proxies.len());
        for entry in &config.trusted_proxies {
            match entry.parse() {
                Ok(network) => trusted.push(network),
                Err(error) => {
                    return Err(crate::config::ConfigErrors::new(vec![
                        crate::config::ConfigError::new(
                            "trusted_proxies",
                            format!("`{entry}` is not an IP address or CIDR block: {error}"),
                        ),
                    ]));
                }
            }
        }
        crate::util::forwarded::set_trusted_proxies(trusted);

        Ok(Self {
            config: Arc::new(config),
            routes: Arc::new(RouteServiceManager::new(route_services)),
            upstreams: Arc::new(upstreams),
            dns: DnsCache::default(),
            breakers: Some(Arc::new(breakers)),
            cache,
            metrics: Arc::new(Metrics::default()),
            ready: Arc::new(AtomicBool::new(true)),
            ca,
        })
    }

    /// Shared readiness flag, read by the admin listener's `/health/ready`.
    ///
    /// True while a backend is selectable, false once an `lb://` pool could not
    /// produce one.
    pub fn readiness(&self) -> Arc<AtomicBool> {
        self.ready.clone()
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

    /// Build the "no upstream" error, recording the failure first.
    fn no_upstream_error(&self, ctx: &mut RequestContext) -> Box<pingora::Error> {
        // A pool that cannot produce a backend means this instance cannot serve.
        self.ready.store(false, Ordering::Relaxed);
        let error: Box<pingora::Error> = GatewayError::ServerNoUpstreamServices.into();
        self.record_upstream_failure(ctx, &error);
        error
    }

    /// Count an upstream failure and record it against the route's breaker.
    ///
    /// Called from every path that ends an upstream attempt without a response:
    /// a connect failure, a proxy error and a `upstream_peer` failure all have to
    /// record a failure, otherwise a half open permit would never be released.
    fn record_upstream_failure(&self, ctx: &mut RequestContext, error: &pingora::Error) {
        let route = ctx.route_id.as_deref().unwrap_or("-");
        self.metrics
            .observe_upstream_error(route, upstream_error_kind(error));

        let Some(breaker_id) = ctx.fallback_id.clone() else {
            return;
        };
        let Some(breakers) = self.breakers.clone() else {
            return;
        };

        tokio::spawn(async move {
            if let Some(service) = breakers.services.get(&breaker_id) {
                service.controller.process(Outcome::Failure).await;
            }
        });
    }

    /// Write a gateway generated response and report the exchange as handled.
    ///
    /// When a filter already produced the response (a cache hit, a rejection, a
    /// redirect) that response *is* the reply: its status, headers and body are
    /// used verbatim. `render_error` only runs for the statuses the proxy maps
    /// from a pingora error, where no filter produced a body.
    async fn respond_directly(
        &self,
        session: &mut Session,
        ctx: &mut RequestContext,
        status: u16,
    ) -> pingora::Result<bool> {
        let (mut header, body) = if let Some(direct) = ctx.direct_response.take() {
            let mut header = ResponseHeader::build(direct.status, None)?;
            for (name, value) in &direct.headers {
                header.insert_header(name.clone(), value.as_str()).ok();
            }
            if !header.headers.contains_key("content-type") {
                header
                    .insert_header("Content-Type", "text/plain; charset=utf-8")
                    .ok();
            }
            (header, direct.body)
        } else {
            let route = ctx.route_id.as_deref();
            match render_error(&self.config.error, status, route, ctx.request_id.as_str()) {
                Ok(rendered) => rendered,
                Err(error) => {
                    error!("failed to build the error response for {status}: {error}");
                    session.respond_error(500).await?;
                    return Ok(true);
                }
            }
        };

        // The correlation id belongs on every generated response.
        if !ctx.request_id.is_empty() && !header.headers.contains_key("x-request-id") {
            let name = self.config.error.request_id_header.clone();
            let id = ctx.request_id.clone();
            header.insert_header(name, id).ok();
        }

        // Gateway generated responses go through the response filters too, so
        // `SecureHeaders` and `AddResponseHeader` apply to errors as well.
        if let Err(error) = self
            .routes
            .filter_phase(
                ctx,
                FilterPhase::ResponseHeader,
                HeaderAndBody::with_resp_header(&mut header),
            )
            .await
        {
            warn!("a response filter failed on a generated response: {error}");
        }

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
    let status = header.status.as_u16();
    // A 204/304 never carries a body, and a HEAD reply carries the headers of the
    // equivalent GET only. Sending one anyway desynchronises the connection.
    let body_allowed = !matches!(status, 204 | 304) && status >= 200;
    let is_head = session.req_header().method.as_str() == "HEAD";

    header.remove_header("Transfer-Encoding");

    if body_allowed {
        header
            .insert_header("Content-Length", body.len().to_string())
            .ok();
    }

    let send_body = body_allowed && !is_head && !body.is_empty();
    let end_of_stream = !send_body;

    if let Err(error) = session
        .write_response_header(Box::new(header), end_of_stream)
        .await
    {
        error!("failed to write the response header: {error}");
        return;
    }

    if send_body && let Err(error) = session.write_response_body(Some(body), true).await {
        error!("failed to write the response body: {error}");
    }
}

/// Request id from the configured header, or a fresh UUID.
///
/// A caller supplied id is echoed into response headers, into the JSON error
/// body and into the access log, so it is restricted to a short run of visible
/// ASCII: anything else could break the header or the JSON document.
fn request_id(header: &RequestHeader, config: &ErrorConfig) -> String {
    header
        .headers
        .get(config.request_id_header.as_str())
        .and_then(|value| value.to_str().ok())
        .and_then(sanitize_request_id)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
}

/// Keep a caller supplied request id only when it is safe to echo.
pub(crate) fn sanitize_request_id(raw: &str) -> Option<String> {
    const MAX_LEN: usize = 128;

    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_LEN {
        return None;
    }

    if !trimmed.chars().all(|ch| ch.is_ascii_graphic()) {
        return None;
    }

    Some(trimmed.to_owned())
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

/// Classify an upstream failure for the `gateway_upstream_errors_total` counter.
fn upstream_error_kind(error: &pingora::Error) -> &'static str {
    match error.etype() {
        pingora::ErrorType::ConnectTimedout
        | pingora::ErrorType::ReadTimedout
        | pingora::ErrorType::WriteTimedout
        | pingora::ErrorType::TLSHandshakeTimedout => "timeout",
        pingora::ErrorType::ConnectError
        | pingora::ErrorType::ConnectProxyFailure
        | pingora::ErrorType::TLSHandshakeFailure => "connect",
        pingora::ErrorType::ReadError | pingora::ErrorType::WriteError => "reset",
        _ => "other",
    }
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

    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<bool> {
        ctx.started = Some(Instant::now());
        ctx.client_ip = client_ip(session);

        // The request id must exist before anything can answer, including the
        // 404 for an unmatched route.
        let header_name = self.config.error.request_id_header.clone();
        let id = request_id(session.req_header(), &self.config.error);
        ctx.request_id = id.clone();
        session.req_header_mut().insert_header(header_name, id).ok();

        // Record the host the client used: `PreserveHostHeader` forwards it and
        // `RewriteLocationResponseHeader` hands it back in `Location`, so an
        // internal upstream host never leaks to the client.
        if ctx.original_host.is_none()
            && let Some(host) = session
                .req_header()
                .headers
                .get("host")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
        {
            ctx.original_host = Some(host);
        }

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
            .filter_phase(
                ctx,
                FilterPhase::RequestHeader,
                HeaderAndBody::with_req_header(header),
            )
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
            let status = ctx
                .direct_response
                .as_ref()
                .map(|r| r.status)
                .unwrap_or(500);
            return self.respond_directly(session, ctx, status).await;
        }

        // Reserve a breaker probe before connecting.
        if let Some(breaker_id) = ctx.fallback_id.clone()
            && let Some(breakers) = self.breakers.as_ref()
            && let Some(service) = breakers.services.get(&breaker_id)
            && let Err(rejection) = service.controller.try_acquire().await
        {
            warn!(breaker = %service.id, "circuit breaker is open, rejecting");
            self.metrics
                .set_circuit_state(&service.id, CircuitState::Open.into());
            // Tell the client when the breaker will probe again instead of a
            // hard coded second.
            let retry_after = rejection.retry_after.as_secs().max(1).to_string();
            ctx.respond_with_empty(503, vec![("Retry-After".to_owned(), retry_after)])
                .ok();
            return self.respond_directly(session, ctx, 503).await;
        }

        self.metrics.record_inflight(1);
        ctx.inflight_counted = true;
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

        let path = session.req_header().uri.path().to_owned();
        let group_key = selection_key(ctx, &path);

        let selection = match route.work {
            RouteWork::LB => {
                let key = group_key.to_be_bytes();
                let Some(addr) = self.upstreams.select(&route_id, &key) else {
                    // A half open probe that never reaches an upstream must still
                    // record a failure, or its permit would leak forever.
                    return Err(self.no_upstream_error(ctx));
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
                let addresses = match self.dns.resolve(&parsed.host, parsed.port).await {
                    Ok(addresses) if !addresses.is_empty() => addresses,
                    Ok(_) => return Err(self.no_upstream_error(ctx)),
                    Err(error) => {
                        warn!(route = %route_id, "upstream name resolution failed: {error}");
                        let failure: Box<pingora::Error> =
                            GatewayError::ServerUpstreamDnsFailure.into();
                        self.record_upstream_failure(ctx, &failure);
                        return Err(failure);
                    }
                };

                // A name with several A records is load balanced, not pinned to
                // the first one: a dead first address must not fail every
                // request until the TTL expires.
                let index = (group_key % addresses.len() as u64) as usize;
                let addr = addresses[index];

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
        let peer = build_peer(
            &selection,
            &self.config.upstream,
            route_config,
            self.ca.as_ref(),
        );
        Ok(Box::new(peer))
    }

    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        // `RewriteLocationResponseHeader` needs the path the *client* asked for;
        // the access log reports it too, so the log shows the original target
        // even after `StripPrefix`/`RewritePath`.
        if ctx.original_request_path.is_none() {
            ctx.original_request_path = upstream_request
                .uri
                .path_and_query()
                .map(|value| value.as_str().to_owned());
        }

        for name in HOP_BY_HOP {
            upstream_request.remove_header(*name);
        }

        if let Some(cache) = self
            .routes
            .local_response_cache_filter(ctx.route_id.as_deref())
        {
            let hit = serve_from_local_cache_if_present(cache, ctx, upstream_request)?;
            self.metrics.record_cache_lookup(hit);
        }

        // Forwarding headers: appended, never overwritten.
        let peer_ip = ctx.client_ip.map(|ip| ip.to_string()).unwrap_or_default();
        if !peer_ip.is_empty() {
            match upstream_request
                .headers
                .get("x-forwarded-for")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
            {
                Some(existing) if !existing.is_empty() => {
                    upstream_request
                        .insert_header("X-Forwarded-For", format!("{existing}, {peer_ip}"))
                        .ok();
                }
                _ => {
                    upstream_request
                        .insert_header("X-Forwarded-For", peer_ip)
                        .ok();
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
            .map(str::to_owned)
        {
            upstream_request
                .insert_header("X-Forwarded-Host", host)
                .ok();
        }

        // Host precedence: an explicit `SetRequestHostHeader`, then a preserved
        // downstream host (only when `PreserveHostHeader` asked for it), then the
        // upstream authority.
        let host = ctx
            .upstream_host_override
            .clone()
            .or_else(|| {
                ctx.preserve_host
                    .then(|| ctx.original_host.clone())
                    .flatten()
            })
            .or_else(|| {
                ctx.selected
                    .as_ref()
                    .map(|selection| selection.host_header.clone())
            });

        if let Some(host) = host {
            upstream_request.insert_header("Host", host).ok();
        }

        Ok(())
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> pingora::Result<()> {
        if let Some(breaker_id) = ctx.fallback_id.clone()
            && let Some(breakers) = self.breakers.as_ref()
            && let Some(service) = breakers.services.get(&breaker_id)
        {
            let outcome = if upstream_response.status.as_u16() >= 500 {
                Outcome::Failure
            } else {
                Outcome::Success
            };
            service.controller.process(outcome).await;

            // Keep the `gateway_circuit_state` gauge in step with the machine.
            let state = service.controller.state().await;
            self.metrics.set_circuit_state(&service.id, state.into());
        }

        // The correlation id belongs on every response the client sees, proxied
        // ones included.
        if !ctx.request_id.is_empty()
            && !upstream_response
                .headers
                .contains_key(self.config.error.request_id_header.as_str())
        {
            let name = self.config.error.request_id_header.clone();
            let id = ctx.request_id.clone();
            upstream_response.insert_header(name, id).ok();
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

        // A response that cannot carry a body must not be re-framed as chunked,
        // and there is nothing for a body filter to rewrite.
        let status = upstream_response.status.as_u16();
        let bodyless = matches!(status, 204 | 304) || status < 200;

        let must_buffer = !bodyless
            && (self
                .routes
                .has_response_body_filters(ctx.route_id.as_deref())
                || ctx.pending_local_response_cache.is_some());

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
                limit,
                seen = ctx.request_body_bytes,
                "request body exceeds the configured limit"
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
    ) -> pingora::Result<Option<Duration>> {
        let Some(chunk) = body.take() else {
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

    fn suppress_error_log(
        &self,
        _session: &Session,
        _ctx: &Self::CTX,
        _error: &pingora::Error,
    ) -> bool {
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
        self.record_upstream_failure(ctx, &e);

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
        self.record_upstream_failure(ctx, &e);
        e
    }

    async fn fail_to_proxy(
        &self,
        session: &mut Session,
        e: &pingora::Error,
        ctx: &mut Self::CTX,
    ) -> FailToProxy {
        // Metrics are recorded once, from `logging`, which pingora calls for
        // every request including the failing ones.
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
            .unwrap_or_else(|| "-".to_owned());

        self.metrics
            .observe_request(route, session.req_header().method.as_str(), status, latency);
        if ctx.inflight_counted {
            self.metrics.record_inflight(-1);
        }

        if self.config.logging.access_log {
            // The path the client asked for, even after `StripPrefix`/
            // `RewritePath` changed what goes upstream.
            let original_path = ctx
                .original_request_path
                .as_deref()
                .map(|value| value.split('?').next().unwrap_or(value))
                .unwrap_or_else(|| session.req_header().uri.path());
            let bytes = ctx.response_body_buffer.len() as u64;

            if let Some(error) = error {
                info!(
                    request_id = %ctx.request_id,
                    client = ?ctx.client_ip,
                    method = %session.req_header().method,
                    path = %original_path,
                    status,
                    route = %route,
                    upstream = %upstream,
                    bytes,
                    latency_ms = latency.as_millis(),
                    error = %error,
                    "access"
                )
            } else {
                info!(
                    request_id = %ctx.request_id,
                    client = ?ctx.client_ip,
                    method = %session.req_header().method,
                    path = %original_path,
                    status,
                    route = %route,
                    upstream = %upstream,
                    bytes,
                    latency_ms = latency.as_millis(),
                    "access"
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_request_id;

    #[test]
    fn accepts_a_normal_request_id() {
        assert_eq!(
            sanitize_request_id(" 8bc9-4db2_abc.1 "),
            Some("8bc9-4db2_abc.1".to_owned())
        );
    }

    #[test]
    fn rejects_ids_that_could_break_a_header_or_a_json_body() {
        assert!(sanitize_request_id("").is_none());
        assert!(sanitize_request_id("   ").is_none());
        assert!(sanitize_request_id("line\nbreak").is_none());
        assert!(sanitize_request_id("tab\there").is_none());
        assert!(sanitize_request_id("h\u{e9}llo").is_none());
        assert!(sanitize_request_id(&"a".repeat(129)).is_none());
    }

    #[test]
    fn a_quote_is_allowed_because_the_error_body_is_escaped() {
        // Visible ASCII is safe: `render_error` escapes it through serde_json.
        assert_eq!(sanitize_request_id("a\"b"), Some("a\"b".to_owned()));
    }

    #[test]
    fn a_generated_id_is_always_accepted() {
        let generated = uuid::Uuid::new_v4().to_string();
        assert_eq!(sanitize_request_id(&generated), Some(generated));
    }
}
