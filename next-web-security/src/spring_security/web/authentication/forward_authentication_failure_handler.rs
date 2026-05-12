use next_web_core::{anys::any_map::AnyMap, traits::http::http_request::HttpRequest};

use crate::web::authentication::authentication_failure_handler::AuthenticationFailureHandler;

#[derive(Clone)]
pub struct ForwardAuthenticationFailureHandler {
    pub(crate) forward_url: Box<str>,
}

impl ForwardAuthenticationFailureHandler {
    pub fn new(forward_url: impl Into<Box<str>>) -> Self {
        let forward_url = forward_url.into();
        assert!(
            forward_url.starts_with("/"),
            "{} is not a valid forward URL",
            forward_url.as_ref()
        );
        Self { forward_url }
    }
}

impl AuthenticationFailureHandler for ForwardAuthenticationFailureHandler {
    fn on_authentication_failure(
        &self,
        request: &axum::extract::Request,
        response: &mut axum::response::Response,
        error: &crate::core::authentication_error::AuthenticationError,
    ) {
        if let Some(any_map) = request.extensions().get::<AnyMap>() {
            block_on(any_map.insert("NEXT_SECURITY_LAST_ERROR".to_string(), error.clone().into()));
            request
                .request_dispatcher(&self.forward_url)
                .map(|dispatcher| dispatcher.forward(request, response));
        };
    }
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(future))
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build Tokio runtime")
            .block_on(future)
    }
}
