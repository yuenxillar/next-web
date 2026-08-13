use std::sync::Arc;

use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
    util::HttpMethod,
};

use crate::{
    oauth2::{
        AuthorizationRequestRepository, HttpSessionOAuth2AuthorizationRequestRepository,
        OAuth2AuthorizationRequestResolver,
    },
    web::{DefaultRedirectStrategy, RedirectStrategy},
};

#[derive(Clone)]
pub struct OAuth2AuthorizationRequestRedirectFilter {
    authorization_request_resolver: Arc<dyn OAuth2AuthorizationRequestResolver>,
    authorization_request_repository: Arc<dyn AuthorizationRequestRepository>,
    authorization_redirect_strategy: Arc<dyn RedirectStrategy>,
}

impl OAuth2AuthorizationRequestRedirectFilter {
    pub const DEFAULT_AUTHORIZATION_REQUEST_BASE_URI: &'static str = "/oauth2/authorization";

    pub fn new(
        authorization_request_resolver: Arc<dyn OAuth2AuthorizationRequestResolver>,
    ) -> Self {
        Self {
            authorization_request_resolver,
            authorization_request_repository: Arc::new(
                HttpSessionOAuth2AuthorizationRequestRepository::default(),
            ),
            authorization_redirect_strategy: Arc::new(DefaultRedirectStrategy::default()),
        }
    }

    pub fn set_authorization_request_repository(
        &mut self,
        authorization_request_repository: Arc<dyn AuthorizationRequestRepository>,
    ) {
        self.authorization_request_repository = authorization_request_repository;
    }

    pub fn set_authorization_redirect_strategy(
        &mut self,
        authorization_redirect_strategy: Arc<dyn RedirectStrategy>,
    ) {
        self.authorization_redirect_strategy = authorization_redirect_strategy;
    }
}

#[async_trait]
impl HttpFilter for OAuth2AuthorizationRequestRedirectFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.method() != HttpMethod::Get {
            return filter_chain.do_filter(request, response).await;
        }

        let Some(authorization_request) = self.authorization_request_resolver.resolve(request)
        else {
            return filter_chain.do_filter(request, response).await;
        };

        let redirect_uri = authorization_request
            .authorization_request_uri()
            .to_string();
        self.authorization_request_repository
            .save_authorization_request(Some(authorization_request), request, response);
        self.authorization_redirect_strategy
            .send_redirect(request, response, &redirect_uri)
            .map_err(FilterError::from)?;

        Ok(())
    }
}

impl Named for OAuth2AuthorizationRequestRedirectFilter {
    fn name(&self) -> &str {
        "OAuth2AuthorizationRequestRedirectFilter"
    }
}
