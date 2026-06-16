use std::sync::Arc;

use next_web_core::{
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
    authorization::AuthenticationManager,
    core::context::security_context_holder::SecurityContextHolder,
    web::authentication::remember_me_services::RememberMeServices,
};

/// Detects a remember-me token in the request (typically a cookie) and
/// auto-authenticates the user via `RememberMeServices`.
#[derive(Clone)]
pub struct RememberMeAuthenticationFilter {
    authentication_manager: Arc<dyn AuthenticationManager>,
    remember_me_services: Arc<dyn RememberMeServices>,
}

impl RememberMeAuthenticationFilter {
    pub fn new(
        authentication_manager: Arc<dyn AuthenticationManager>,
        remember_me_services: Arc<dyn RememberMeServices>,
    ) -> Self {
        Self {
            authentication_manager,
            remember_me_services,
        }
    }
}

#[async_trait]
impl HttpFilter for RememberMeAuthenticationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        // Only attempt remember-me if no authentication already exists
        let has_auth = SecurityContextHolder::get_context()
            .and_then(|ctx| ctx.get_authentication())
            .is_some();

        if !has_auth {
            if let Some(remember_me_auth) = self.remember_me_services.auto_login(request, response)
            {
                // Authenticate the remember-me token via the AuthenticationManager
                match self
                    .authentication_manager
                    .authenticate(remember_me_auth.as_ref())
                {
                    Ok(auth_result) => match SecurityContextHolder::get_context() {
                        Some(ctx) => ctx.set_authentication(Some(auth_result)),
                        None => {
                            let ctx = SecurityContextHolder::create_empty_context();
                            ctx.set_authentication(Some(auth_result));
                            SecurityContextHolder::set_context(ctx);
                        }
                    },
                    Err(_) => {
                        // Remember-me failed — continue without authentication
                    }
                }
            }
        }

        filter_chain.do_filter(request, response).await
    }
}

impl Named for RememberMeAuthenticationFilter {
    fn name(&self) -> &str {
        "RememberMeAuthenticationFilter"
    }
}
