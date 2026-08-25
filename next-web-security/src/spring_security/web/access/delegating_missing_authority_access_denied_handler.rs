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
    core::{AuthenticationError, AuthenticationErrorKind},
    web::{
        access::{AccessDeniedHandler, AccessDeniedHandlerImpl},
        authentication::DelegatingAuthenticationEntryPointBuilder,
        savedrequest::RequestCache,
        util::matcher::AnyRequestMatcher,
        AuthenticationEntryPoint, WebAttributes,
    },
};

/// An AccessDeniedHandler that adapts AuthenticationEntryPoints based on missing
/// GrantedAuthoritys. These authorities are specified in an AuthorityAuthorizationDecision inside an AuthorizationDeniedException.
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

    /// Use this AccessDeniedHandler for AccessDeniedError that this handler doesn't support.
    /// By default, this uses AccessDeniedHandlerImpl.
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
        let denied = match err {
            AccessDeniedError::AuthorizationDenied(authorization_denied_error) => {
                authorization_denied_error
            }
            _ => return Vec::new(),
        };

        let authorization_result = denied.authorization_result();
        match (authorization_result as &dyn Any).downcast_ref::<FactorAuthorizationDecision>() {
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
            None => match (authorization_result as &dyn Any)
                .downcast_ref::<AuthorityAuthorizationDecision>()
            {
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
            .flat_map(|entry| entry.error().cloned())
            .collect::<Vec<_>>();

        for authority_error in error_entries.iter() {
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

                    let err = AuthenticationError::with_kind(
                        format!("Missing Authorities {}", required_authority),
                        AuthenticationErrorKind::InsufficientAuthentication,
                    );
                    entry_point.commence(request, response, &err)?;
                    return Ok(());
                }
                None => continue,
            }
        }

        self.default_access_denied_handler
            .handle(request, response, denied)
    }
}

/// A builder for configuring the set of authority/entry-point pairs
#[derive(Clone, Default)]
pub struct DelegatingMissingAuthorityAccessDeniedHandlerBuilder {
    entry_point_builder_by_authority: BTreeMap<String, DelegatingAuthenticationEntryPointBuilder>,
}

impl DelegatingMissingAuthorityAccessDeniedHandlerBuilder {
    /// Use this AuthenticationEntryPoint when the given missingAuthority is missing from the authenticated user
    pub fn add_entry_point_for(
        &mut self,
        entry_point: Arc<dyn AuthenticationEntryPoint>,
        missing_authority: impl Into<String>,
    ) -> &mut Self {
        let mut builder = DelegatingAuthenticationEntryPointBuilder::default();
        builder.add_entry_point_for(entry_point, AnyRequestMatcher::instance());
        self.entry_point_builder_by_authority
            .insert(missing_authority.into(), builder);
        self
    }

    /// Use this AuthenticationEntryPoint when the given missingAuthority is missing from the authenticated user
    pub fn add_entry_point_for_with_builder<F>(
        &mut self,
        entry_point_builder_fn: F,
        missing_authority: impl Into<String>,
    ) -> &mut Self
    where
        F: FnOnce(&mut DelegatingAuthenticationEntryPointBuilder),
    {
        let missing_authority = missing_authority.into();
        entry_point_builder_fn(
            self.entry_point_builder_by_authority
                .entry(missing_authority)
                .or_insert_with(|| DelegatingAuthenticationEntryPointBuilder::default()),
        );
        self
    }

    pub fn build(&mut self) -> DelegatingMissingAuthorityAccessDeniedHandler {
        let entry_point_by_authority: BTreeMap<String, Arc<dyn AuthenticationEntryPoint>> = self
            .entry_point_builder_by_authority
            .iter_mut()
            .map(|(key, value)| (key.clone(), value.build()))
            .collect::<_>();
        DelegatingMissingAuthorityAccessDeniedHandler::new(entry_point_by_authority)
    }
}

/// A mapping of a GrantedAuthority.get_authority() to a possibly None RequiredFactorError.
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

    fn error(&self) -> Option<&RequiredFactorError> {
        self.error.as_ref()
    }

    fn authority(&self) -> &str {
        &self.authority
    }
}
