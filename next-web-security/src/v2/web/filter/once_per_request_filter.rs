use next_web_core::{
    any::AnyValue,
    async_trait,
    error::BoxError,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpRequest, HttpResponse,
    },
    Named, Required,
};

use crate::core::util::nameable::Nameable;
use tracing::{debug, trace};

use crate::web::filter::{
    advice_filter::AdviceFilterExt, path_matching_filter::PathMatchingFilterExt,
};

#[derive(Clone)]
pub struct OncePerRequestFilter {
    name: Option<String>,
    enabled: bool,
    filter_once_per_request: bool,
}

impl OncePerRequestFilter {
    const ALREADY_FILTERED_SUFFIX: &str = ".FILTERED";

    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn set_name<T: ToString>(&mut self, name: T) {
        self.name = Some(name.to_string())
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled
    }

    pub fn is_filter_once_per_request(&self) -> bool {
        self.filter_once_per_request
    }

    pub fn set_filter_once_per_request(&mut self, filter_once_per_request: bool) {
        self.filter_once_per_request = filter_once_per_request
    }

    pub fn get_already_filtered_attribute_name(&self) -> String {
        format!(
            "{}{}",
            self.get_name().unwrap_or(std::any::type_name::<Self>()),
            Self::ALREADY_FILTERED_SUFFIX
        )
    }
}

impl Default for OncePerRequestFilter {
    fn default() -> Self {
        Self {
            name: None,
            enabled: true,
            filter_once_per_request: Default::default(),
        }
    }
}

#[async_trait]
pub trait OncePerRequestFilterExt: Send + Sync {
    async fn do_filter_internal(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        chain: &dyn HttpFilterChain,
    ) -> Result<(), BoxError>;
}

#[derive(Clone)]
pub struct HttpFilterWrapper<T>(pub T);

impl<T> Named for HttpFilterWrapper<T>
where
    T: Named + Required<OncePerRequestFilter>,
{
    fn name(&self) -> &str {
        self.0.name()
    }
}

impl<T> Nameable for HttpFilterWrapper<T>
where
    T: Named + Required<OncePerRequestFilter>,
{
    fn set_name(&mut self, name: &str) {
        self.0.get_mut_object().set_name(name)
    }
}

#[async_trait]
impl<T> HttpFilter for HttpFilterWrapper<T>
where
    T: Clone + 'static,
    T: Required<OncePerRequestFilter> + Named,
    T: AdviceFilterExt,
    T: PathMatchingFilterExt,
{
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        let _self = self.0.get_object();
        let already_filtered_attribute_name = _self.get_already_filtered_attribute_name();
        trace!(
            "already_filtered_attribute_name: {}",
            already_filtered_attribute_name
        );

        if request
            .get_attribute(&already_filtered_attribute_name)
            .is_some()
            && _self.is_filter_once_per_request()
        {
            trace!(
                "Filter '{:?}' already executed.  Proceeding without invoking this filter.",
                _self.get_name()
            );

            filter_chain.do_filter(request, response).await
        } else if !_self.is_enabled() {
            debug!("Filter '{:?}' is not enabled for the current request.  Proceeding without invoking this filter.",
                    _self.get_name());
            filter_chain.do_filter(request, response).await
        } else {
            trace!(
                "Filter '{:?}' not yet executed.  Executing now.",
                _self.get_name()
            );

            request.set_attribute(&already_filtered_attribute_name, AnyValue::Boolean(true));
            self.0
                .do_filter_internal(request, response, filter_chain)
                .await?;

            // Once the request has finished, we're done and we don't
            // need to mark as 'already filtered' any more.
            request.remove_attribute(&already_filtered_attribute_name);

            Ok(())
        }
    }

    fn set_name(&mut self, name: &str) {
        self.0.get_mut_object().set_name(name)
    }

    fn apply_login_url(&mut self, login_url: &str) -> bool {
        // The wrapper is the filter the chain runs; the filter that answers the
        // login page is the one it wraps, so the property is applied there.
        self.0.apply_login_url(login_url)
    }
}

#[async_trait]
impl<T> OncePerRequestFilterExt for T
where
    T: AdviceFilterExt + PathMatchingFilterExt,
{
    async fn do_filter_internal(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        chain: &dyn HttpFilterChain,
    ) -> Result<(), BoxError> {
        let continue_chain = self.pre_handle(request, response, Some(self)).await;

        if !continue_chain {
            // A filter that does not continue the chain has answered the request
            // itself — a redirect to the login page, an unauthorized status — so
            // the request is over and not failed. Apache Shiro's `AdviceFilter`
            // returns here as well.
            return Ok(());
        }

        let mut error: Option<Box<dyn std::error::Error + Send + Sync>> = None;
        if let Err(err) = chain.do_filter(request, response).await {
            error = Some(err.into());
        }

        if let Err(err) = self.post_handle(request, response).await {
            error = Some(err);
        }
        trace!("Successfully invoked postHandle method");

        self.cleanup(request, response, error).await
    }
}
