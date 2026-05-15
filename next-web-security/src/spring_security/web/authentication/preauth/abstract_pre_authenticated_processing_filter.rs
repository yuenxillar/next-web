use std::sync::Arc;

use axum::{extract::Request, response::Response};
use next_web_core::{
    anys::{any_map::AnyMap, any_value::AnyValue},
    error::BoxError,
};

use crate::{
    authorization::authentication_manager::AuthenticationManager,
    core::{
        authentication::Authentication,
        authentication_error::AuthenticationError,
        context::{security_context::SecurityContext, security_context_holder::SecurityContextHolder},
        filter::Filter,
    },
};

pub const NEXT_SECURITY_AUTHENTICATION: &str = "NEXT_SECURITY_AUTHENTICATION";
pub const NEXT_SECURITY_REQUEST_ATTRIBUTES: &str = "NEXT_SECURITY_REQUEST_ATTRIBUTES";

#[derive(Clone)]
pub struct AbstractPreAuthenticatedProcessingFilterSupport {
    authentication_manager: Arc<dyn AuthenticationManager>,
    continue_filter_chain_on_unsuccessful_authentication: bool,
}

impl AbstractPreAuthenticatedProcessingFilterSupport {
    pub fn new(authentication_manager: Arc<dyn AuthenticationManager>) -> Self {
        Self {
            authentication_manager,
            continue_filter_chain_on_unsuccessful_authentication: true,
        }
    }

    pub fn set_continue_filter_chain_on_unsuccessful_authentication(&mut self, value: bool) {
        self.continue_filter_chain_on_unsuccessful_authentication = value;
    }

    pub fn authenticate(
        &self,
        request: &Request,
        response: &mut Response,
        authentication: &dyn Authentication,
    ) -> Result<Option<Arc<dyn Authentication>>, BoxError> {
        match self.authentication_manager.authenticate(authentication) {
            Ok(authentication) => {
                self.successful_authentication(request, response, authentication.clone())?;
                Ok(Some(authentication))
            }
            Err(error) => {
                self.unsuccessful_authentication(request, error.clone())?;
                if self.continue_filter_chain_on_unsuccessful_authentication {
                    Ok(None)
                } else {
                    Err(Box::new(error))
                }
            }
        }
    }

    fn successful_authentication(
        &self,
        request: &Request,
        _response: &mut Response,
        authentication: Arc<dyn Authentication>,
    ) -> Result<(), BoxError> {
        let mut context = SecurityContext::new(None);
        context.set_authentication(Some(authentication.clone()));
        SecurityContextHolder::set_context(context);

        if let Some(any_map) = request.extensions().get::<AnyMap>() {
            block_on(any_map.insert(
                NEXT_SECURITY_AUTHENTICATION.to_string(),
                AnyValue::Object(Box::new(authentication)),
            ));
        }

        Ok(())
    }

    fn unsuccessful_authentication(
        &self,
        request: &Request,
        error: AuthenticationError,
    ) -> Result<(), BoxError> {
        SecurityContextHolder::clear_context();
        if let Some(any_map) = request.extensions().get::<AnyMap>() {
            block_on(any_map.insert(
                "NEXT_SECURITY_LAST_ERROR".to_string(),
                AnyValue::Object(Box::new(error)),
            ));
        }
        Ok(())
    }
}

impl Filter for AbstractPreAuthenticatedProcessingFilterSupport {
    fn do_filter(&self, _req: &mut Request, _res: &mut Response) -> Result<(), BoxError> {
        Ok(())
    }
}

pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
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
