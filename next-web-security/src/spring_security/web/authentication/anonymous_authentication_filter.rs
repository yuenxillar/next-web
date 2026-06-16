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

use crate::core::{
    authority_utils::AuthorityUtils, context::security_context_holder::SecurityContextHolder,
    Authentication,
};

/// Detects when no `Authentication` object is present in the `SecurityContext`
/// and populates it with an `AnonymousAuthenticationToken` so that downstream
/// filters can assume a populated `SecurityContext`.
#[derive(Clone)]
pub struct AnonymousAuthenticationFilter {
    key: String,
    principal: String,
    authorities: Vec<String>,
}

impl AnonymousAuthenticationFilter {
    pub fn new(key: String, principal: String, authorities: Vec<String>) -> Self {
        Self {
            key,
            principal,
            authorities,
        }
    }

    pub fn get_key(&self) -> &str {
        &self.key
    }

    pub fn get_principal(&self) -> &str {
        &self.principal
    }

    pub fn get_authorities(&self) -> &[String] {
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
        // Only set anonymous auth if no authentication exists in the current context
        let need_anonymous = match SecurityContextHolder::get_context() {
            Some(ctx) => ctx.get_authentication().is_none(),
            None => true, // No context exists yet, create one
        };

        if need_anonymous {
            let anonymous_token =
                crate::authentication::anonymous_authentication_token::AnonymousAuthenticationToken::new(
                    &self.key,
                    &self.principal,
                    AuthorityUtils::create_authority_list(
                        self.authorities.iter().map(|s| s.as_str()),
                    ),
                );
            let auth: Arc<dyn Authentication> = Arc::new(anonymous_token);

            match SecurityContextHolder::get_context() {
                Some(ctx) => ctx.set_authentication(Some(auth)),
                None => {
                    let ctx = SecurityContextHolder::create_empty_context();
                    ctx.set_authentication(Some(auth));
                    SecurityContextHolder::set_context(ctx);
                }
            }
        }

        filter_chain.do_filter(request, response).await
    }
}

impl Named for AnonymousAuthenticationFilter {
    fn name(&self) -> &str {
        "AnonymousAuthenticationFilter"
    }
}
