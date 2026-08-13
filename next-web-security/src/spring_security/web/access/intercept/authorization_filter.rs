use std::sync::Arc;

use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::{
    access::intercept::RequestAuthorizationContext,
    authorization::{AuthorizationDeniedError, AuthorizationEventPublisher, AuthorizationManager},
    core::{
        context::{SecurityContextHolder, SecurityContextHolderStrategy},
        Authentication,
    },
};

/// An authorization filter that restricts access to the URL using AuthorizationManager.
#[derive(Clone)]
pub struct AuthorizationFilter {
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    authorization_manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    event_publisher: Option<Arc<dyn AuthorizationEventPublisher>>,
}

impl AuthorizationFilter {
    /// Creates an instance.
    pub fn new(
        authorization_manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    ) -> Self {
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            authorization_manager,
            event_publisher: None,
        }
    }

    /// Sets the SecurityContextHolderStrategy to use.
    /// The default action is to use the SecurityContextHolderStrategy stored in SecurityContextHolder.
    pub fn set_security_context_holder_strategy(
        &mut self,
        strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = strategy;
    }

    fn get_authentication(&self) -> Result<Arc<dyn Authentication>, &'static str> {
        match self
            .security_context_holder_strategy
            .get_context()
            .and_then(|ctx| ctx.get_authentication().cloned())
        {
            Some(authentication) => Ok(authentication),
            None => Err("An Authentication object was not found in the SecurityContext"),
        }
    }

    /// Use this AuthorizationEventPublisher to publish AuthorizationDeniedEvents and AuthorizationGrantedEvents.
    pub fn set_authorization_event_publisher(
        &mut self,
        event_publisher: Arc<dyn AuthorizationEventPublisher>,
    ) {
        self.event_publisher = Some(event_publisher);
    }

    /// Gets the AuthorizationManager used by this filter
    pub fn get_authorization_manager(
        &self,
    ) -> &Arc<dyn AuthorizationManager<RequestAuthorizationContext>> {
        &self.authorization_manager
    }
}

#[async_trait]
impl HttpFilter for AuthorizationFilter {
    async fn do_filter(
        &self,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        let authentication = self.get_authentication().map_err(FilterError::custom)?;
        let mut context = RequestAuthorizationContext::from(req as &dyn HttpRequest);
        let result = self
            .authorization_manager
            .authorize(authentication.as_ref(), &mut context)
            .await;

        if let Some(publisher) = self.event_publisher.as_ref() {
            publisher.publish_authorization_event(
                authentication.clone(),
                Box::new(context),
                result.clone(),
            );
        }

        if let Some(result) = result.as_ref() {
            if !result.is_granted() {
                return Err(FilterError::Chain(Box::new(AuthorizationDeniedError::new(
                    "Access Denied",
                    *result,
                ))));
            }
        }

        return filter_chain.do_filter(req, resp).await;
    }
}

impl Named for AuthorizationFilter {
    fn name(&self) -> &str {
        "AuthorizationFilter"
    }
}
