use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use pingora::http::ResponseHeader;
use pingora::prelude::*;
use pingora::proxy::{FailToProxy, ProxyHttp};
use pingora::upstreams::peer::HttpPeer;
use tracing::{error, warn};

use crate::circuit_breaker::circuit_breaker_service_manager::CircuitBreakerServiceManager;
use crate::circuit_breaker::circuit_state::CircuitState;
use crate::context::{HeaderAndBody, RequestContext};
use crate::error::GatewayError;
use crate::filter::factory::local_response_cache::{
    capture_response_for_local_cache, serve_from_local_cache_if_present, store_local_cache_entry,
    LocalResponseCacheManager,
};
use crate::properties::gateway_properties::GatewayApplicationProperties;
use crate::handler::predicate::RouteServiceManager;
use crate::service::route_service::RouteWork;
use std::sync::Arc;

#[derive(Clone)]
pub struct NextGatewayApplication {
    application_properties: GatewayApplicationProperties,
    route_service_manager: RouteServiceManager,
    circuit_breaker_service_manager: Option<CircuitBreakerServiceManager>,
    local_response_cache_manager: Option<Arc<LocalResponseCacheManager>>,
    jingyue_service: crate::service::jingyue_service::JingYueService,
}

impl NextGatewayApplication {
    pub fn new(
        application_properties: GatewayApplicationProperties,
        route_service_manager: RouteServiceManager,
        circuit_breaker_service_manager: Option<CircuitBreakerServiceManager>,
    ) -> Self {
        Self {
            local_response_cache_manager: application_properties
                .local_response_cache_enabled()
                .then(LocalResponseCacheManager::shared),
            application_properties,
            route_service_manager,
            circuit_breaker_service_manager,
            jingyue_service: crate::service::jingyue_service::JingYueService::default(),
        }
    }
}

#[async_trait]
impl ProxyHttp for NextGatewayApplication {
    type CTX = RequestContext;

    fn new_ctx<'a>(&self) -> Self::CTX {
        Self::CTX {
            fallback_id: None,
            route_id: None,
            original_request_path: None,
            buffer_response_body: false,
            response_body_buffer: Vec::new(),
            local_response_cache_manager: self.local_response_cache_manager.clone(),
            local_response_cache_request: None,
            pending_local_response_cache: None,
            session: None,
            direct_response: None,
        }
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        // Directly return the assertion operation error for the request implementation
        let route_predicate_result = self.route_service_manager.predicate(session);

        if !route_predicate_result.allowable {
            return Err(GatewayError::ServerRejectsRequest.into());
        };

        let sevice_name = route_predicate_result.service_name;
        let fallback_id = route_predicate_result.fallback_id;

        // Is the routing fuse in open or half open position
        if !fallback_id.is_empty() {
            if let Some(circuit_breaker_service_manager) =
                self.circuit_breaker_service_manager.as_ref()
            {
                if let Some(service) = circuit_breaker_service_manager.services.get(fallback_id) {
                    ctx.fallback_id = Some(fallback_id.into());
                    if let CircuitState::Open = service.controller.state().await {
                        return Err(GatewayError::ServerRejectsRequest.into());
                    }
                }
            }
        }

        // Determine whether it is a normal address or a service name
        let normal = sevice_name.contains(".");

        let route_work = route_predicate_result.work;

        ctx.route_id = Some(route_predicate_result.route_id.into());

        let client_metadata = route_predicate_result
            .metadata
            .as_ref()
            .map(|v| v.client.as_ref());

        // Request upstream through routing working mode
        match route_work {
            &RouteWork::Http | &RouteWork::LB => {
                let mut http_peer: HttpPeer = if normal {
                    HttpPeer::new(sevice_name, false, "".into())
                } else {
                    // Choose appropriate upstream services
                    if let Some(service) =
                        self.jingyue_service.select(sevice_name, route_work).await
                    {
                        HttpPeer::new(service.addr(), false, service.name().to_string())
                    } else {
                        return Err(GatewayError::ServerNoUpstreamServices.into());
                    }
                };

                if let Some(metadata) = client_metadata {
                    if let Some(client) = metadata {
                        set_request_timeout(
                            &mut http_peer,
                            client.connect_timeout,
                            client.read_timeout,
                            client.write_timeout,
                        );
                    }
                }
                return Ok(Box::new(http_peer));
            }
            &RouteWork::Https => {
                let mut http_peer = HttpPeer::new(sevice_name, true, "".into());
                if let Some(metadata) = client_metadata {
                    if let Some(client) = metadata {
                        set_request_timeout(
                            &mut http_peer,
                            client.connect_timeout,
                            client.read_timeout,
                            client.write_timeout,
                        );
                    }
                }

                return Ok(Box::new(http_peer));
            }
        }
    }

    // upstream request filter
    async fn upstream_request_filter(
        &self,
        _session: &mut Session,
        upstream_request_header: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        // Keep the original downstream path so response filters can inspect it later.
        ctx.original_request_path = upstream_request_header
            .uri
            .path_and_query()
            .map(|value| value.as_str().to_string())
            .or_else(|| Some(upstream_request_header.uri.path().to_string()));

        if let Some(filter) = self
            .route_service_manager
            .local_response_cache_filter(ctx.route_id.as_deref())
        {
            serve_from_local_cache_if_present(filter, ctx, upstream_request_header)?;
        }

        self.route_service_manager
            .filter(ctx, HeaderAndBody::with_req_header(upstream_request_header))
            .await?;
        Ok(())
    }

    async fn response_filter(
        &self,
        _session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()>
    where
        Self::CTX: Send + Sync,
    {
        if let Some(id) = ctx.fallback_id.as_ref() {
            if let Some(manager) = self.circuit_breaker_service_manager.as_ref() {
                if let Some(service) = manager.services.get(id) {
                    service.controller.process(true).await;
                }
            }
        }

        self.route_service_manager
            .filter(ctx, HeaderAndBody::with_resp_header(upstream_response))
            .await?;

        if let Some(filter) = self
            .route_service_manager
            .local_response_cache_filter(ctx.route_id.as_deref())
        {
            capture_response_for_local_cache(filter, ctx, upstream_response);
        }

        let should_buffer_response_body = self
            .route_service_manager
            .has_response_body_filters(ctx.route_id.as_deref())
            || ctx.pending_local_response_cache.is_some();

        if should_buffer_response_body {
            ctx.buffer_response_body = true;
            ctx.response_body_buffer.clear();
            upstream_response.remove_header("Content-Length");
            upstream_response.remove_header("Transfer-Encoding");
            upstream_response
                .insert_header("Transfer-Encoding", "Chunked")
                .ok();
        }

        Ok(())
    }

    fn response_body_filter(
        &self,
        _session: &mut Session,
        body: &mut Option<Bytes>,
        end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> Result<Option<Duration>>
    where
        Self::CTX: Send + Sync,
    {
        if !ctx.buffer_response_body {
            return Ok(None);
        }

        if let Some(chunk) = body.take() {
            ctx.response_body_buffer.extend_from_slice(&chunk);
        }

        if !end_of_stream {
            return Ok(None);
        }

        *body = Some(Bytes::from(std::mem::take(&mut ctx.response_body_buffer)));
        self.route_service_manager
            .filter_blocking(ctx, HeaderAndBody::with_resp_body(body))?;
        store_local_cache_entry(ctx, body.as_ref());
        ctx.buffer_response_body = false;

        Ok(None)
    }

    fn suppress_error_log(&self, _session: &Session, _ctx: &Self::CTX, _error: &Error) -> bool {
        true
    }

    /// Triggered when a proxy error occurs after establishing a connection with the upstream server
    ///(such as unexpected disconnection of the upstream connection, response parsing failure, etc.)
    fn error_while_proxy(
        &self,
        peer: &HttpPeer,
        session: &mut Session,
        e: Box<Error>,
        ctx: &mut Self::CTX,
        client_reused: bool,
    ) -> Box<Error> {
        if let Some(id) = ctx.fallback_id.clone() {
            if let Some(manager) = self.circuit_breaker_service_manager.clone() {
                tokio::spawn(async move {
                    if let Some(service) = manager.services.get(&id) {
                        service.controller.process(false).await;
                    }
                });
            }
        }

        let mut e = e.more_context(format!("Peer: {}", peer));
        // only reused client connections where retry buffer is not truncated
        e.retry
            .decide_reuse(client_reused && !session.as_ref().retry_buffer_truncated());
        e
    }

    fn fail_to_connect(
        &self,
        _session: &mut Session,
        _peer: &HttpPeer,
        ctx: &mut Self::CTX,
        e: Box<Error>,
    ) -> Box<Error> {
        if let Some(id) = ctx.fallback_id.clone() {
            if let Some(manager) = self.circuit_breaker_service_manager.clone() {
                tokio::spawn(async move {
                    if let Some(service) = manager.services.get(&id) {
                        service.controller.process(false).await;
                    }
                });
            }
        } else {
            warn!("fail_to_connect called without fallback id");
        }
        e
    }

    async fn fail_to_proxy(
        &self,
        session: &mut Session,
        e: &Error,
        ctx: &mut Self::CTX,
    ) -> FailToProxy
    where
        Self::CTX: Send + Sync,
    {
        if let Some(direct_response) = ctx.direct_response.take() {
            let response_code = direct_response.status;
            let mut response =
                match ResponseHeader::build(response_code, Some(direct_response.headers.len() + 1))
                {
                    Ok(response) => response,
                    Err(build_error) => {
                        error!("failed to build direct response header: {build_error}");
                        session
                            .respond_error(response_code)
                            .await
                            .unwrap_or_else(|write_error| {
                                error!("failed to send fallback direct response: {write_error}");
                            });

                        return FailToProxy {
                            error_code: response_code,
                            can_reuse_downstream: false,
                        };
                    }
                };

            // Materialize the synthetic response collected during request filtering.
            for (name, value) in direct_response.headers {
                if let Err(header_error) = response.append_header(name, value.as_str()) {
                    error!("failed to insert direct response header: {header_error}");
                }
            }

            if let Err(length_error) = response.set_content_length(direct_response.body.len()) {
                error!("failed to set direct response content length: {length_error}");
            }

            if let Err(write_error) = session
                .write_response_header(Box::new(response), false)
                .await
            {
                error!("failed to write direct response header: {write_error}");
            } else if let Err(write_error) = session
                .write_response_body(Some(direct_response.body), true)
                .await
            {
                error!("failed to write direct response body: {write_error}");
            }

            return FailToProxy {
                error_code: response_code,
                can_reuse_downstream: false,
            };
        }

        let code = match e.etype() {
            HTTPStatus(code) => *code,
            _ => match e.esource() {
                ErrorSource::Upstream => 502,
                ErrorSource::Downstream => match e.etype() {
                    WriteError | ReadError | ConnectionClosed => 0,
                    _ => 400,
                },
                ErrorSource::Internal | ErrorSource::Unset => 500,
            },
        };

        if code > 0 {
            session
                .respond_error(code)
                .await
                .unwrap_or_else(|write_error| {
                    error!("failed to send error response to downstream: {write_error}");
                });
        }

        FailToProxy {
            error_code: code,
            can_reuse_downstream: false,
        }
    }
}

pub fn set_request_timeout(
    http: &mut HttpPeer,
    connection_timeout: Option<u64>,
    read_timeout: Option<u64>,
    write_timeout: Option<u64>,
) {
    connection_timeout.map(|v| http.options.connection_timeout = Some(Duration::from_millis(v)));
    read_timeout.map(|v| http.options.read_timeout = Some(Duration::from_millis(v)));
    write_timeout.map(|v| http.options.write_timeout = Some(Duration::from_millis(v)));
}
