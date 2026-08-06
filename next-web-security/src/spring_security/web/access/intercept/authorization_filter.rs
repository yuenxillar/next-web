use std::sync::Arc;

use next_web_core::{
    anys::any_value::AnyValue,
    async_trait,
    error::BoxError,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::{
    access::intercept::request_authorization_context::RequestAuthorizationContext,
    authorization::{AuthorizationEventPublisher, AuthorizationManager},
    core::{
        context::{SecurityContextHolder, SecurityContextHolderStrategy},
        Authentication,
    },
};

#[derive(Clone)]
pub struct AuthorizationFilter {
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    authorization_manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    event_publisher: Option<Arc<dyn AuthorizationEventPublisher>>,

    observe_once_per_request: bool,
    filter_error_dispatch: bool,
    filter_async_dispatch: bool,
}

impl AuthorizationFilter {
    pub fn new(
        authorization_manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    ) -> Self {
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            authorization_manager,
            event_publisher: None,
            observe_once_per_request: false,
            filter_error_dispatch: true,
            filter_async_dispatch: true,
        }
    }

    pub fn set_observe_once_per_request(&mut self, observe_once_per_request: bool) {
        self.observe_once_per_request = observe_once_per_request;
    }

    pub fn set_filter_error_dispatch(&mut self, filter_error_dispatch: bool) {
        self.filter_error_dispatch = filter_error_dispatch;
    }

    pub fn set_filter_async_dispatch(&mut self, filter_async_dispatch: bool) {
        self.filter_async_dispatch = filter_async_dispatch;
    }

    pub fn set_authorization_event_publisher(
        &mut self,
        event_publisher: Arc<dyn AuthorizationEventPublisher>,
    ) {
        self.event_publisher = Some(event_publisher);
    }

    pub fn set_security_context_holder_strategy(
        &mut self,
        strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = strategy;
    }

    fn is_applied(&self, req: &mut dyn HttpRequest) -> bool {
        req.get_attribute(&self.get_already_filtered_attribute_name())
            .is_some()
    }

    fn skip_dispatch(&self, req: &mut dyn HttpRequest) -> bool {
        todo!()
    }

    fn get_already_filtered_attribute_name(&self) -> String {
        format!("{}.APPLIED", self.name())
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
}

#[async_trait]
impl HttpFilter for AuthorizationFilter {
    async fn do_filter(
        &self,
        req: &mut dyn HttpRequest,
        resp: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if self.observe_once_per_request && self.is_applied(req) {
            return filter_chain.do_filter(req, resp).await;
        }

        if self.skip_dispatch(req) {
            return filter_chain.do_filter(req, resp).await;
        }

        let already_filtered_attribute_name = self.get_already_filtered_attribute_name();
        req.set_attribute(&already_filtered_attribute_name, AnyValue::Boolean(true));

        let authentication = self.get_authentication().map_err(Into::<BoxError>::into)?;

        let var = todo!();
        let result = self
            .authorization_manager
            .authorize(authentication.as_ref(), var)
            .await;
        self.event_publisher.as_ref().map(|publisher| {
            publisher.publish_authorization_event(authentication, todo!(), result)
        });

        if result.as_ref().map(|s| s.is_granted()).unwrap_or(true) {
            return Err(FilterError::Custom("Access Denied".into()));
        }

        filter_chain.do_filter(req, resp).await;
        req.remove_attribute(&already_filtered_attribute_name);

        Ok(())
    }
}

impl Named for AuthorizationFilter {
    fn name(&self) -> &str {
        "AuthorizationFilter"
    }
}
