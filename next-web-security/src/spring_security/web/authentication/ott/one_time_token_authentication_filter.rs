use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use crate::web::authentication::{
    base_authentication_processing_filter::BaseAuthenticationProcessingFilter,
    ott::OneTimeTokenAuthenticationConverter, BaseAuthenticationProcessingFilterExt,
};

#[derive(Clone)]
pub struct OneTimeTokenAuthenticationFilter {
    base: BaseAuthenticationProcessingFilter,
}

impl Deref for OneTimeTokenAuthenticationFilter {
    type Target = BaseAuthenticationProcessingFilter;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for OneTimeTokenAuthenticationFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl OneTimeTokenAuthenticationFilter {
    pub const DEFAULT_LOGIN_PROCESSING_URL: &'static str = "/login/ott";
}

impl BaseAuthenticationProcessingFilterExt for OneTimeTokenAuthenticationFilter {}

#[async_trait]
impl HttpFilter for OneTimeTokenAuthenticationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        BaseAuthenticationProcessingFilter::do_filter(request, response, filter_chain, self).await
    }
}

impl Named for OneTimeTokenAuthenticationFilter {
    fn name(&self) -> &str {
        "OneTimeTokenAuthenticationFilter"
    }
}

impl Default for OneTimeTokenAuthenticationFilter {
    fn default() -> Self {
        let mut base = BaseAuthenticationProcessingFilter::new(Self::DEFAULT_LOGIN_PROCESSING_URL);
        base.set_authentication_converter(Arc::new(OneTimeTokenAuthenticationConverter::default()));

        Self { base }
    }
}
