use std::sync::Arc;

use axum::{extract::Request, http::StatusCode};

use crate::{
    access::intercept::request_authorization_context::RequestAuthorizationContext,
    authorization::authorization_manager::AuthorizationManager,
    core::{authentication::Authentication, filter::Filter},
};

pub struct AuthorizationFilter {
    authorization_manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    observe_once_per_request: bool,
    filter_error_dispatch: bool,
    filter_async_dispatch: bool,
}

impl AuthorizationFilter {
    pub fn new(
        authorization_manager: Arc<dyn AuthorizationManager<RequestAuthorizationContext>>,
    ) -> Self {
        Self {
            authorization_manager,
            observe_once_per_request: false,
            filter_error_dispatch: true,
            filter_async_dispatch: true,
        }
    }
}

impl Filter for AuthorizationFilter {
    fn do_filter(
        &self,
        _req: &mut axum::extract::Request,
        res: &mut axum::response::Response,
    ) -> Result<(), next_web_core::error::BoxError> {
        let decision = block_on(
            self.authorization_manager
                .check(Box::new(AnonymousAuthentication), RequestAuthorizationContext {}),
        );

        if decision.map(|decision| decision.is_granted()).unwrap_or(false) {
            Ok(())
        } else {
            *res.status_mut() = StatusCode::FORBIDDEN;
            *res.body_mut() = "Forbidden".to_string().into();
            Ok(())
        }
    }
}

struct AnonymousAuthentication;

impl Authentication for AnonymousAuthentication {
    fn is_authenticated(&self) -> bool {
        false
    }

    fn is_anonymous(&self) -> bool {
        true
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
