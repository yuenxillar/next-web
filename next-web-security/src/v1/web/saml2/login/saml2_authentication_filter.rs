use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpRequest, HttpResponse,
    },
    Named,
};

use crate::web::{
    authentication::{
        BaseAuthenticationProcessingFilter, BaseAuthenticationProcessingFilterExt,
    },
    saml2::login::Saml2AuthenticationConverter,
};

/// The assertion consumer service: the endpoint an asserting party posts to.
///
/// The filter turns the submission into a [`Saml2AuthenticationToken`] through
/// its converter, and leaves the decision about the response to the provider.
#[derive(Clone)]
pub struct Saml2AuthenticationFilter {
    base: BaseAuthenticationProcessingFilter,
}

impl Deref for Saml2AuthenticationFilter {
    type Target = BaseAuthenticationProcessingFilter;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Saml2AuthenticationFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Saml2AuthenticationFilter {
    /// The endpoint an assertion is posted to, by default.
    pub const DEFAULT_LOGIN_PROCESSING_URL: &'static str = "/login/saml2/sso/{registrationId}";
}

impl BaseAuthenticationProcessingFilterExt for Saml2AuthenticationFilter {}

#[async_trait]
impl HttpFilter for Saml2AuthenticationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        BaseAuthenticationProcessingFilter::do_filter(request, response, filter_chain, self).await
    }
}

impl Named for Saml2AuthenticationFilter {
    fn name(&self) -> &str {
        "Saml2AuthenticationFilter"
    }
}

impl Default for Saml2AuthenticationFilter {
    fn default() -> Self {
        let mut base =
            BaseAuthenticationProcessingFilter::new(Self::DEFAULT_LOGIN_PROCESSING_URL);
        base.set_authentication_converter(Arc::new(Saml2AuthenticationConverter));

        Self { base }
    }
}

#[cfg(test)]
mod tests {
    use next_web_core::Named;

    use super::*;

    #[test]
    fn the_default_processing_url_carries_the_registration_id() {
        assert_eq!(
            Saml2AuthenticationFilter::DEFAULT_LOGIN_PROCESSING_URL,
            "/login/saml2/sso/{registrationId}"
        );
    }

    #[test]
    fn the_filter_is_named_after_the_assertion_consumer_service() {
        let filter = Saml2AuthenticationFilter::default();

        assert_eq!(filter.name(), "Saml2AuthenticationFilter");
    }
}
