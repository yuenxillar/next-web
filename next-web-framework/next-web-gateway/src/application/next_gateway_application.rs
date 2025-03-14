use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use pingora::http::ResponseHeader;
use pingora::prelude::*;
use pingora::proxy::ProxyHttp;
use pingora::upstreams::peer::HttpPeer;

use crate::circuit_breaker::circuit_breaker_service_manager::CircuitBreakerServiceManager;
use crate::circuit_breaker::circuit_state::CircuitState;
use crate::error::gateway_error::GatewayError;
use crate::properties::gateway_properties::GatewayApplicationProperties;
use crate::route::route_service_manager::{RouteServiceManager, RouteWork};

#[derive(Clone)]
pub struct NextGatewayApplication {
    application_properties: GatewayApplicationProperties,
    route_service_manager: RouteServiceManager,
    circuit_breaker_service_manager: Option<CircuitBreakerServiceManager>,
    jingyue_service_discovery: crate::service_discovery::JingYueServiceDiscovery,
}

impl NextGatewayApplication {
    pub fn new(
        application_properties: GatewayApplicationProperties,
        route_service_manager: RouteServiceManager,
        circuit_breaker_service_manager: Option<CircuitBreakerServiceManager>,
    ) -> Self {
        Self {
            application_properties,
            route_service_manager,
            circuit_breaker_service_manager,
            jingyue_service_discovery: crate::service_discovery::JingYueServiceDiscovery {},
        }
    }
}

#[async_trait]
impl ProxyHttp for NextGatewayApplication {
    type CTX = ApplicationContext;

    fn new_ctx<'a>(&self) -> Self::CTX {
        Self::CTX {
            fallback_id: None,
            route_id: None,
            session: None,
        }
    }

    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        // Directly return the assertion operation error for the request implementation
        let predicate = self.route_service_manager.predicate(session);

        let predicate_result = predicate.result;
        if !predicate_result {
            return Err(GatewayError::ServerRejectsRequest.into());
        };

        let sevice_name = predicate.service_name;
        let fallback_id = predicate.fallback_id;

        // Is the routing fuse in open or half open position
        if !fallback_id.is_empty() {
            if let Some(circuit_breaker_service_manager) = &self.circuit_breaker_service_manager {
                if let Some(service) = circuit_breaker_service_manager.services.get(fallback_id) {
                    ctx.fallback_id = Some(fallback_id.into());
                    if let CircuitState::Open = service.controller.state() {
                        return Err(GatewayError::ServerRejectsRequest.into());
                    }
                }
            }
        }

        // Determine whether it is a normal address or a service name
        let normal = sevice_name.contains(".");

        let route_work = predicate.work;

        ctx.route_id = Some(predicate.route_id.into());

        // Request upstream through routing working mode
        match route_work {
            &RouteWork::Http | &RouteWork::LB => {
                let mut http_peer: HttpPeer = if normal {
                    HttpPeer::new(sevice_name, false, "".into())
                } else {
                    // Choose appropriate upstream services
                    if let Some(service) = self
                        .jingyue_service_discovery
                        .select(sevice_name, route_work)
                        .await
                    {
                        HttpPeer::new(service.addr(), false, service.name().to_string())
                    } else {
                        return Err(GatewayError::ServerNoUpstreamServices.into());
                    }
                };

                set_request_timeout(&mut http_peer, std::time::Duration::from_millis(500));
                return Ok(Box::new(http_peer));
            }
            &RouteWork::Https => {
                // TODO: Implement HTTPS routing
                let mut http_peer = HttpPeer::new(sevice_name, false, "".into());
                set_request_timeout(&mut http_peer, std::time::Duration::from_millis(500));

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
        self.route_service_manager.filter(
            ctx,
            upstream_request_header,
            &mut ResponseHeader::build(200, None)?,
        );
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
        // Success recored
        ctx.fallback_id.as_ref().map(|id| {
            self.circuit_breaker_service_manager
                .as_ref()
                .map(|m| m.services.get(id).map(|s| s.controller.process(true)))
        });

        self.route_service_manager.filter(
            ctx,
            &mut RequestHeader::build("GET", &[47], None)?,
            upstream_response,
        );
        Ok(())
    }

    fn response_body_filter(
        &self,
        _session: &mut Session,
        _body: &mut Option<Bytes>,
        _end_of_stream: bool,
        _ctx: &mut Self::CTX,
    ) -> Result<Option<Duration>>
    where
        Self::CTX: Send + Sync,
    {
        Ok(None)
    }

    // 是否抑制日志的输出
    fn suppress_error_log(&self, _session: &Session, _ctx: &Self::CTX, _error: &Error) -> bool {
        true
    }

    // 当与上游服务器建立连接后出现代理错误时触发（如上游连接意外断开、响应解析失败等）
    fn error_while_proxy(
        &self,
        peer: &HttpPeer,
        session: &mut Session,
        e: Box<Error>,
        ctx: &mut Self::CTX,
        client_reused: bool,
    ) -> Box<Error> {
        // Error record
        ctx.fallback_id.as_ref().map(|id| {
            self.circuit_breaker_service_manager
                .as_ref()
                .map(|m| m.services.get(id).map(|s| s.controller.process(false)))
        });

        let mut e = e.more_context(format!("Peer: {}", peer));
        // only reused client connections where retry buffer is not truncated
        e.retry
            .decide_reuse(client_reused && !session.as_ref().retry_buffer_truncated());
        e
    }

    // 当连接上游服务器失败时触发（如DNS解析失败、TCP连接超时等）
    fn fail_to_connect(
        &self,
        _session: &mut Session,
        _peer: &HttpPeer,
        ctx: &mut Self::CTX,
        e: Box<Error>,
    ) -> Box<Error> {
        // Error record
        ctx.fallback_id.as_ref().map(|id| {
            self.circuit_breaker_service_manager
                .as_ref()
                .map(|m| m.services.get(id).map(|s| s.controller.process(false)))
        });
        e
    }
}

#[derive(Clone)]
pub struct ApplicationContext {
    pub fallback_id: Option<String>,
    pub route_id: Option<String>,
    pub session: Option<String>
}

pub fn set_request_timeout(http: &mut HttpPeer, timeout: Duration) {
    http.options.connection_timeout = Some(timeout);
    http.options.read_timeout = Some(timeout);
}
