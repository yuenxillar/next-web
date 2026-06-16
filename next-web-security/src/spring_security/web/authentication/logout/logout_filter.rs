use std::sync::Arc;

use crate::{
    core::context::{
        security_context_holder::SecurityContextHolder,
        security_context_holder_strategy::SecurityContextHolderStrategy,
    },
    web::{
        authentication::logout::{
            CompositeLogoutHandler, LogoutHandler, LogoutSuccessHandler,
            SimpleUrlLogoutSuccessHandler,
        },
        util::matcher::{PathPatternRequestMatcher, RequestMatcher},
    },
};
use next_web_core::{
    async_trait,
    error::BoxError,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
    util::StringUtils,
};
use tracing::debug;
use tracing::Level;

#[derive(Clone)]
pub struct LogoutFilter {
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,

    logout_request_matcher: Arc<dyn RequestMatcher>,
    handler: Arc<dyn LogoutHandler>,
    logout_success_handler: Arc<dyn LogoutSuccessHandler>,
}

impl LogoutFilter {
    pub fn new(
        logout_success_handler: Arc<dyn LogoutSuccessHandler>,
        handlers: impl IntoIterator<Item = Arc<dyn LogoutHandler>>,
    ) -> Self {
        let logout_request_matcher =
            Arc::new(PathPatternRequestMatcher::path_pattern(None, "/logout"));
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            handler: Arc::new(CompositeLogoutHandler::new(
                handlers.into_iter().collect::<Vec<_>>(),
            )),
            logout_request_matcher,
            logout_success_handler,
        }
    }

    pub fn with_logout_success_url(
        logout_success_url: impl AsRef<str>,
        handlers: impl IntoIterator<Item = Arc<dyn LogoutHandler>>,
    ) -> Self {
        let logout_request_matcher =
            Arc::new(PathPatternRequestMatcher::path_pattern(None, "/logout"));

        let mut url_logout_success_handler = SimpleUrlLogoutSuccessHandler::default();

        let logout_success_url = logout_success_url.as_ref();
        if StringUtils::has_text(logout_success_url) {
            url_logout_success_handler.set_default_target_url(logout_success_url);
        }

        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            handler: Arc::new(CompositeLogoutHandler::new(
                handlers.into_iter().collect::<Vec<_>>(),
            )),
            logout_success_handler: Arc::new(url_logout_success_handler),
            logout_request_matcher,
        }
    }

    fn requires_logout(&self, request: &mut dyn HttpRequest) -> bool {
        if self.logout_request_matcher.matches(request) {
            return true;
        }

        if tracing::enabled!(Level::TRACE) {
            tracing::trace!("Did not match request to {:?}", self.logout_request_matcher);
        }

        false
    }

    pub fn set_security_context_holder_strategy(
        &mut self,
        security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = security_context_holder_strategy;
    }

    pub fn set_logout_request_matcher(&mut self, logout_request_matcher: Arc<dyn RequestMatcher>) {
        self.logout_request_matcher = logout_request_matcher;
    }

    pub fn set_filter_processes_url(&mut self, filter_processes_url: &str) {
        self.logout_request_matcher = Arc::new(PathPatternRequestMatcher::path_pattern(
            None,
            filter_processes_url,
        ));
    }
}

#[async_trait]
impl HttpFilter for LogoutFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if self.requires_logout(request) {
            if let Some(auth) = self
                .security_context_holder_strategy
                .get_context()
                .map(|ctx| ctx.get_authentication())
            {
                if tracing::enabled!(Level::DEBUG) {
                    debug!("Logging out [{:?}]", auth.as_ref().map(|s| s.get_name()));
                }

                let auth = auth.as_deref();
                self.handler.logout(request, response, auth).await;
                self.logout_success_handler
                    .on_logout_success(request, response, auth)
                    .await?;

                return Ok(());
            }
        }

        filter_chain.do_filter(request, response).await
    }
}

impl Named for LogoutFilter {
    fn name(&self) -> &str {
        "LogoutFilter"
    }
}
