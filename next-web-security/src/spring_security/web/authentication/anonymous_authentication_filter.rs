use std::{any::Any, sync::Arc};

use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
    util::StringUtils,
};
use tracing::{debug, trace};

use crate::{
    authentication::{
        authentication_details_source::AuthenticationDetailsSource, AnonymousAuthenticationToken,
    },
    core::{
        authority::AuthorityUtils,
        context::{SecurityContext, SecurityContextHolder, SecurityContextHolderStrategy},
        Authentication, GrantedAuthority,
    },
    web::authentication::{AuthPrincipal, WebAuthenticationDetailsSource},
};

/// Detects when no `Authentication` object is present in the `SecurityContext`
/// and populates it with an `AnonymousAuthenticationToken` so that downstream
/// filters can assume a populated `SecurityContext`.
#[derive(Clone)]
pub struct AnonymousAuthenticationFilter {
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    authentication_details_source: Arc<dyn AuthenticationDetailsSource>,

    key: String,
    principal: AuthPrincipal,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
}

impl AnonymousAuthenticationFilter {
    pub fn new(
        key: impl Into<String>,
        principal: AuthPrincipal,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        let security_context_holder_strategy = SecurityContextHolder::get_context_holder_strategy();
        let key = key.into();
        assert!(StringUtils::has_text(key.as_str()), "key cannot be  empty");

        Self {
            key,

            authentication_details_source: Arc::new(WebAuthenticationDetailsSource::default()),
            security_context_holder_strategy,
            principal,
            authorities,
        }
    }

    /// Creates a filter with a principal named "anonymousUser" and the single authority "ROLE_ANONYMOUS".
    pub fn with_key(key: impl Into<String>) -> Self {
        Self::new(
            key,
            Arc::new("anonymousUser".to_string()),
            AuthorityUtils::create_authority_list(["ROLE_ANONYMOUS"]),
        )
    }

    pub fn after_properties_set(&self) {
        assert!(
            StringUtils::has_text(self.key.as_str()),
            "key must have length"
        );
        assert!(
            !self.authorities.is_empty(),
            "authorities must not be empty"
        );
    }

    pub fn get_key(&self) -> &str {
        &self.key
    }

    fn default_with_anonymous(
        &self,
        req: &dyn HttpRequest,
        current_context: Arc<dyn SecurityContext>,
    ) -> Arc<dyn SecurityContext> {
        let anonymous_context = || {
            let anonymous = self.create_authentication(req);
            debug!("Set SecurityContextHolder to anonymous SecurityContext");

            let anonymous_context = self.security_context_holder_strategy.create_empty_context();
            anonymous_context.set_authentication(Some(anonymous));

            anonymous_context
        };

        match current_context.get_authentication() {
            Some(_auth) => trace!("Did not set SecurityContextHolder since already authenticated."),
            None => return anonymous_context(),
        };

        current_context
    }

    pub fn create_authentication(&self, request: &dyn HttpRequest) -> Arc<dyn Authentication> {
        let mut token = AnonymousAuthenticationToken::new(
            &self.key,
            self.principal.to_owned(),
            self.authorities.to_owned(),
        );
        let details = self.authentication_details_source.build_details(request);
        token.set_details(Some(details));

        Arc::new(token)
    }

    pub fn set_authentication_details_source(
        &mut self,
        authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    ) {
        self.authentication_details_source = authentication_details_source;
    }

    /// Sets the SecurityContextHolderStrategy to use.
    /// The default action is to use the SecurityContextHolderStrategy stored in SecurityContextHolder.
    pub fn set_security_context_holder_strategy(
        &mut self,
        strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = strategy;
    }

    pub fn get_principal(&self) -> &dyn Any {
        self.principal.as_ref()
    }

    pub fn get_authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        &self.authorities
    }
}

#[async_trait]
impl HttpFilter for AnonymousAuthenticationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        let current_context = self
            .default_with_anonymous(request, self.security_context_holder_strategy.get_context());
        self.security_context_holder_strategy
            .set_context(current_context);
        filter_chain.do_filter(request, response).await
    }
}

impl Named for AnonymousAuthenticationFilter {
    fn name(&self) -> &str {
        "AnonymousAuthenticationFilter"
    }
}
