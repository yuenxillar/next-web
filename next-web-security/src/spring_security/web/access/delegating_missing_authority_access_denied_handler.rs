use std::{any::Any, collections::BTreeMap, sync::Arc};

use next_web_core::{
    anys::any_value::AnyValue,
    error::BoxError,
    traits::http::{http_request::HttpRequest, http_response::HttpResponse},
};

use crate::{
    access::AccessDeniedError,
    authorization::{
        AuthorityAuthorizationDecision, FactorAuthorizationDecision, RequiredFactor,
        RequiredFactorError,
    },
    web::{
        access::{AccessDeniedHandler, AccessDeniedHandlerImpl},
        authentication::DelegatingAuthenticationEntryPointBuilder,
        savedrequest::RequestCache,
        AuthenticationEntryPoint, WebAttributes,
    },
};

#[derive(Clone)]
pub struct DelegatingMissingAuthorityAccessDeniedHandler {
    entry_points: BTreeMap<String, Arc<dyn AuthenticationEntryPoint>>,
    request_cache: Option<Arc<dyn RequestCache>>,
    default_access_denied_handler: Arc<dyn AccessDeniedHandler>,
}

impl DelegatingMissingAuthorityAccessDeniedHandler {
    pub fn new(entry_points: BTreeMap<String, Arc<dyn AuthenticationEntryPoint>>) -> Self {
        Self {
            entry_points,
            request_cache: None,
            default_access_denied_handler: Arc::new(AccessDeniedHandlerImpl::default()),
        }
    }

    /// Use this AccessDeniedHandler for AccessDeniedError that this handler doesn't support. By default, this uses AccessDeniedHandlerImpl.
    pub fn set_default_access_denied_handler(&mut self, handler: Arc<dyn AccessDeniedHandler>) {
        self.default_access_denied_handler = handler;
    }

    /// Use this RequestCache to remember the current request.
    /// Uses None by default.
    pub fn set_request_cache(&mut self, request_cache: Arc<dyn RequestCache>) {
        self.request_cache = Some(request_cache);
    }

    pub fn builder() -> DelegatingMissingAuthorityAccessDeniedHandlerBuilder {
        DelegatingMissingAuthorityAccessDeniedHandlerBuilder::default()
    }

    fn authority_errors(&self, err: &AccessDeniedError) -> Vec<AuthorityRequiredFactorErrorEntry> {
        let denied = match err.authorization_denied_error() {
            Some(denied) => denied.authorization_result(),
            None => return Default::default(),
        };

        let denied = denied as &dyn Any;
        match denied.downcast_ref::<FactorAuthorizationDecision>() {
            Some(factor_decision) => {
                return factor_decision
                    .factor_errors()
                    .iter()
                    .map(|err| {
                        AuthorityRequiredFactorErrorEntry::new(
                            err.required_factor().authority(),
                            Some(err.to_owned()),
                        )
                    })
                    .collect();
            }
            None => match denied.downcast_ref::<AuthorityAuthorizationDecision>() {
                Some(authority_decision) => authority_decision
                    .authorities()
                    .iter()
                    .filter_map(|ga| ga.authority())
                    .map(|authority| {
                        if authority.starts_with("FACTOR_") {
                            let required = RequiredFactor::with_authority(authority).build();
                            AuthorityRequiredFactorErrorEntry::new(
                                authority,
                                Some(RequiredFactorError::create_missing(required)),
                            )
                        } else {
                            AuthorityRequiredFactorErrorEntry::new(authority, None)
                        }
                    })
                    .collect(),
                None => return Vec::new(),
            },
        }
    }
}

impl AccessDeniedHandler for DelegatingMissingAuthorityAccessDeniedHandler {
    fn handle(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        denied: &AccessDeniedError,
    ) -> Result<(), BoxError> {
        let error_entries = self.authority_errors(denied);

        let errors = error_entries
            .iter()
            .map(|entry| entry.error())
            .filter_map(|opt| opt)
            .collect::<Vec<_>>();

        for authority_error in error_entries {
            let required_authority = authority_error.authority();
            match self.entry_points.get(required_authority) {
                Some(entry_point) => {
                    self.request_cache
                        .as_ref()
                        .map(|req_cache| req_cache.save_request(request, response));

                    if !errors.is_empty() {
                        request.set_attribute(
                            WebAttributes::REQUIRED_FACTOR_ERRORS,
                            AnyValue::Object(Box::new(errors)),
                        );
                    }

                    // let err = InsufficientAuthenticationError::new(
                    //     format!("Missing Authorities {}", required_authority),
                    //     denied,
                    // );
                    // entry_point.commence(request, response, Some(err))?;
                    // return Ok(());
                    todo!()
                }
                None => continue,
            }
        }

        // self.default_access_denied_handler
        //     .handle(request, response, denied);
        // Ok(())

        todo!()
    }
}

#[derive(Clone, Default)]
pub struct DelegatingMissingAuthorityAccessDeniedHandlerBuilder {
    entry_point_builder_by_authority: BTreeMap<String, ()>,
}

impl DelegatingMissingAuthorityAccessDeniedHandlerBuilder {
    pub fn add_entry_point_for(
        &mut self,
        entry_point: Arc<dyn AuthenticationEntryPoint>,
        missing_authority: impl Into<String>,
    ) -> &mut Self {
        self.entry_point_builder_by_authority
            .insert(missing_authority.into(), ());

        self
    }

    pub fn add_entry_point_for_with_builder<F>(
        &mut self,
        entry_point_builder_fn: F,
        missing_authority: impl Into<String>,
    ) -> &mut Self
    where
        F: FnOnce(&mut DelegatingAuthenticationEntryPointBuilder),
    {
        self
    }

    pub fn build(&self) -> DelegatingMissingAuthorityAccessDeniedHandler {
        // let entry_points: BTreeMap<String, Box<dyn AuthenticationEntryPoint>> = self
        //     .entry_point_builder_by_authority
        //     .iter()
        //     .map(|(k, _)| (k.clone(),))
        //     .collect();
        // DelegatingMissingAuthorityAccessDeniedHandler::new(entry_points)
        //
        todo!()
    }
}

struct AuthorityRequiredFactorErrorEntry {
    authority: String,
    error: Option<RequiredFactorError>,
}

impl AuthorityRequiredFactorErrorEntry {
    pub fn new(authority: impl Into<String>, err: Option<RequiredFactorError>) -> Self {
        Self {
            authority: authority.into(),
            error: err,
        }
    }

    fn error(&self) -> Option<RequiredFactorError> {
        todo!()
    }

    fn authority(&self) -> &str {
        &self.authority
    }
}
