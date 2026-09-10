use std::{any::TypeId, collections::HashSet, sync::Arc};

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
use tracing::{debug, enabled, trace, Level};

use crate::{
    authentication::{
        authentication_details_source::AuthenticationDetailsSource, AnonymousAuthenticationToken,
    },
    authorization::AuthenticationManager,
    core::{
        context::{SecurityContextHolder, SecurityContextHolderStrategy},
        Authentication, AuthenticationError,
    },
    web::{
        authentication::{
            www::BasicAuthenticationConverter, AuthenticationConverter, RememberMeServices,
        },
        authentication_entry_point::AuthenticationEntryPoint,
        context::{RequestAttributeSecurityContextRepository, SecurityContextRepository},
    },
};

/// Processes a HTTP request's BASIC authorization headers, putting the result into the
/// `SecurityContextHolder`.
///
/// For a detailed background on what this filter is designed to process, refer to
/// [RFC 1945, Section 11.1](https://tools.ietf.org/html/rfc1945). Any realm name
/// presented in the HTTP request is ignored.
///
/// In summary, this filter is responsible for processing any request that has a HTTP
/// request header of `Authorization` with an authentication scheme of `Basic` and a
/// Base64-encoded `username:password` token. For example, to authenticate user "Aladdin"
/// with password "open sesame" the following header would be presented:
///
/// ```text
/// Authorization: Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ==
/// ```
///
/// This filter can be used to provide BASIC authentication services to both remoting
/// protocol clients (such as Hessian and SOAP) as well as standard user agents (such as
/// Internet Explorer and Netscape).
///
/// If authentication is successful, the resulting `Authentication` object will be placed
/// into the `SecurityContextHolder`.
///
/// If authentication fails and `ignore_failure` is `false` (the default), an
/// `AuthenticationEntryPoint` implementation is called (unless the `ignore_failure`
/// property is set to `true`). Usually this should be `BasicAuthenticationEntryPoint`,
/// which will prompt the user to authenticate again via BASIC authentication.
///
/// Basic authentication is an attractive protocol because it is simple and widely
/// deployed. However, it still transmits a password in clear text and as such is
/// undesirable in many situations.
///
/// Note that if a `RememberMeServices` is set, this filter will automatically send back
/// remember-me details to the client. Therefore, subsequent requests will not need to
/// present a BASIC authentication header as they will be authenticated using the
/// remember-me mechanism.
#[derive(Clone)]
pub struct BasicAuthenticationFilter {
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    authentication_entry_point: Option<Arc<dyn AuthenticationEntryPoint>>,
    authentication_manager: Arc<dyn AuthenticationManager>,
    remember_me_services: Option<Arc<dyn RememberMeServices>>,
    ignore_failure: bool,
    credentials_charset: String,
    authentication_converter: Box<dyn AuthenticationConverter>,
    security_context_repository: Arc<dyn SecurityContextRepository>,
    mfa_enabled: bool,

    ext: Option<Arc<dyn BasicAuthenticationFilterExt>>,
}

impl BasicAuthenticationFilter {
    /// Creates an instance which will authenticate against the supplied
    /// `AuthenticationManager` and which will ignore failed authentication attempts,
    /// allowing the request to proceed down the filter chain.
    ///
    /// # Arguments
    ///
    /// * `authentication_manager` - The bean to submit authentication requests to.
    pub fn with_ignore_failure(authentication_manager: Arc<dyn AuthenticationManager>) -> Self {
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            authentication_entry_point: None,
            authentication_manager,
            remember_me_services: None,
            ignore_failure: true,
            credentials_charset: "UTF-8".to_string(),
            authentication_converter: Box::new(BasicAuthenticationConverter::default()),
            security_context_repository: Arc::new(
                RequestAttributeSecurityContextRepository::default(),
            ),
            mfa_enabled: false,
            ext: None,
        }
    }

    /// Creates an instance which will authenticate against the supplied
    /// `AuthenticationManager` and use the supplied `AuthenticationEntryPoint` to handle
    /// authentication failures.
    ///
    /// # Arguments
    ///
    /// * `authentication_manager` - The bean to submit authentication requests to.
    /// * `authentication_entry_point` - Will be invoked when authentication fails.
    ///   Typically an instance of `BasicAuthenticationEntryPoint`.
    pub fn new(
        authentication_manager: Arc<dyn AuthenticationManager>,
        authentication_entry_point: Arc<dyn AuthenticationEntryPoint>,
    ) -> Self {
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            authentication_entry_point: Some(authentication_entry_point),
            authentication_manager,
            remember_me_services: None,
            ignore_failure: false,
            credentials_charset: "UTF-8".to_string(),
            authentication_converter: Box::new(BasicAuthenticationConverter::default()),
            security_context_repository: Arc::new(
                RequestAttributeSecurityContextRepository::default(),
            ),
            mfa_enabled: false,
            ext: None,
        }
    }

    /// Sets the `SecurityContextRepository` to save the `SecurityContext` on
    /// authentication success. The default action is not to save the `SecurityContext`.
    ///
    /// # Arguments
    ///
    /// * `security_context_repository` - The `SecurityContextRepository` to use. Cannot
    ///   be null.
    pub fn set_security_context_repository(
        &mut self,
        security_context_repository: Arc<dyn SecurityContextRepository>,
    ) {
        self.security_context_repository = security_context_repository;
    }

    /// Enables Multi-Factor Authentication (MFA) support.
    ///
    /// # Arguments
    ///
    /// * `mfa_enabled` - `true` to enable MFA support, `false` to disable it. Default is
    ///   `false`.
    pub fn set_mfa_enabled(&mut self, mfa_enabled: bool) {
        self.mfa_enabled = mfa_enabled;
    }

    /// Sets the `AuthenticationConverter` to use. Defaults to
    /// `BasicAuthenticationConverter`.
    ///
    /// # Arguments
    ///
    /// * `authentication_converter` - The converter to use.
    pub fn set_authentication_converter(
        &mut self,
        authentication_converter: Box<dyn AuthenticationConverter>,
    ) {
        self.authentication_converter = authentication_converter;
    }

    /// Sets the `BasicAuthenticationFilterExt` to use.
    pub fn set_basic_authentication_filter_ext<T>(&mut self, ext: T)
    where
        T: BasicAuthenticationFilterExt,
        T: 'static,
    {
        self.ext = Some(Arc::new(ext));
    }

    /// Validates that required properties are set after construction.
    pub fn after_properties_set(&self) {
        // authentication_manager is always set in constructors, so no null check needed here.
        if !self.is_ignore_failure() {
            assert!(
                self.authentication_entry_point.is_some(),
                "An AuthenticationEntryPoint is required"
            );
        }
    }

    /// Determines whether MFA authority merging should be performed.
    fn should_perform_mfa(
        &self,
        current: Option<&dyn Authentication>,
        authentication_result: &dyn Authentication,
    ) -> bool {
        if !self.mfa_enabled {
            return false;
        }
        let current = match current {
            Some(c) => c,
            None => return false,
        };
        if !current.is_authenticated() {
            return false;
        }

        current.name() == authentication_result.name()
    }

    /// Determines whether re-authentication is required for the given username.
    ///
    /// Only re-authenticate if username doesn't match SecurityContextHolder and user
    /// isn't authenticated (see SEC-53). Also handles the unusual condition where an
    /// `AnonymousAuthenticationToken` is already present (see SEC-610).
    pub fn authentication_is_required(&self, username: &str) -> bool {
        let ctx = self.security_context_holder_strategy.as_ref().get_context();
        let existing_auth = match ctx.get_authentication() {
            Some(auth) => auth,
            None => return true,
        };
        if existing_auth.name() != username || !existing_auth.is_authenticated() {
            return true;
        }

        // Handle unusual condition where an AnonymousAuthenticationToken is already
        // present. This shouldn't happen very often, as BasicAuthenticationFilter is
        // meant to
        // be earlier in the filter chain than AnonymousAuthenticationFilter.
        // Nevertheless, presence of both an AnonymousAuthenticationToken together with a
        // BASIC authentication request header should indicate reauthentication using the
        // BASIC protocol is desirable. This behaviour is also consistent with that
        // provided by form and digest, both of which force re-authentication if the
        // respective header is detected (and in doing so replace/ any existing
        // AnonymousAuthenticationToken). See SEC-610.
        existing_auth.of() == TypeId::of::<AnonymousAuthenticationToken>()
    }

    /// Returns the authentication entry point, if set.
    pub fn get_authentication_entry_point(&self) -> Option<&Arc<dyn AuthenticationEntryPoint>> {
        self.authentication_entry_point.as_ref()
    }

    /// Returns the authentication manager.
    pub fn get_authentication_manager(&self) -> &Arc<dyn AuthenticationManager> {
        &self.authentication_manager
    }

    /// Returns whether failures are ignored.
    pub fn is_ignore_failure(&self) -> bool {
        self.ignore_failure
    }

    /// Sets the `SecurityContextHolderStrategy` to use. The default action is to use the
    /// `SecurityContextHolderStrategy` stored in `SecurityContextHolder`.
    pub fn set_security_context_holder_strategy(
        &mut self,
        security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = security_context_holder_strategy;
    }

    /// Sets the `AuthenticationDetailsSource` to use. By default, it is set to use the
    /// `WebAuthenticationDetailsSource`. Note that this configuration applies exclusively
    /// when the `authentication_converter` is set to `BasicAuthenticationConverter`. If
    /// you are utilizing a different implementation, you will need to manually specify
    /// the authentication details on it.
    pub fn set_authentication_details_source(
        &mut self,
        authentication_details_source: Arc<dyn AuthenticationDetailsSource>,
    ) {
        if let Some(basic_converter) = self
            .authentication_converter
            .as_any_mut()
            .downcast_mut::<BasicAuthenticationConverter>()
        {
            basic_converter.set_authentication_details_source(authentication_details_source);
        }
    }

    /// Sets the `RememberMeServices` to use.
    pub fn set_remember_me_services(&mut self, remember_me_services: Arc<dyn RememberMeServices>) {
        self.remember_me_services = Some(remember_me_services);
    }

    /// Returns the credentials charset.
    pub fn get_credentials_charset(&self, _http_request: &dyn HttpRequest) -> &str {
        &self.credentials_charset
    }
}

#[async_trait]
impl HttpFilter for BasicAuthenticationFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        let auth_request = match self.authentication_converter.convert(request)? {
            Some(auth_request) => auth_request,
            None => {
                trace!("Did not process authentication request since failed to find username and password in Basic Authorization header");
                return filter_chain.do_filter(request, response).await;
            }
        };

        let username = auth_request.name();
        trace!(
            "Found username '{}' in Basic Authorization header",
            username
        );

        if self.authentication_is_required(username.as_ref()) {
            let mut func = async || -> Result<(), AuthenticationError> {
                let mut auth_result = self
                    .authentication_manager
                    .authenticate(auth_request.as_ref())
                    .await?;

                let current = SecurityContextHolder::get_context().get_authentication();

                if self.should_perform_mfa(current.as_deref(), auth_result.as_ref()) {
                    let mut builder = auth_result.as_ref().to_builder();
                    builder.authorities(Box::new(move |authorities| {
                        let new_authorities: HashSet<&str> =
                            authorities.iter().filter_map(|a| a.authority()).collect();

                        let to_extend: Vec<_> = current
                            .as_ref()
                            .expect("current_authority is None")
                            .authorities()
                            .iter()
                            .filter(|ca| {
                                !ca.authority()
                                    .is_some_and(|auth| new_authorities.contains(auth))
                            })
                            .cloned()
                            .collect();

                        authorities.extend(to_extend);
                    }));
                    auth_result = builder.build();
                }

                let context = self.security_context_holder_strategy.create_empty_context();
                context.set_authentication(Some(auth_result.clone()));
                self.security_context_holder_strategy
                    .set_context(context.clone());
                if enabled!(Level::DEBUG) {
                    tracing::debug!("Set SecurityContextHolder to {}", auth_result.name());
                }
                if let Some(remember_me_services) = &self.remember_me_services {
                    remember_me_services
                        .login_success(request, response, auth_result.as_ref())
                        .await;
                }
                self.security_context_repository
                    .save_context(&context, request, response)
                    .await;

                if let Some(ext) = self.ext.as_ref() {
                    ext.on_successful_authentication(request, response, auth_result.as_ref())
                        .await
                        .map_err(|err| AuthenticationError::new(err.to_string()))?;
                }
                Ok(())
            };

            match func().await {
                Ok(_) => {}
                Err(err) => {
                    self.security_context_holder_strategy.clear_context();
                    debug!("Failed to process authentication request: {}", err);
                    if let Some(remember_me_services) = self.remember_me_services.as_ref() {
                        remember_me_services.login_fail(request, response);
                    }

                    if let Some(ext) = self.ext.as_ref() {
                        ext.on_unsuccessful_authentication(request, response, &err)
                            .await?;
                    }
                    if self.ignore_failure || self.authentication_entry_point.is_none() {
                        return filter_chain.do_filter(request, response).await;
                    } else {
                        if let Some(entry_point) = self.authentication_entry_point.as_ref() {
                            entry_point.commence(request, response, &err)?;
                        }
                    }

                    return Ok(());
                }
            };
        }

        filter_chain.do_filter(request, response).await
    }
}

impl Named for BasicAuthenticationFilter {
    fn name(&self) -> &str {
        "BasicAuthenticationFilter"
    }
}

#[async_trait]
pub trait BasicAuthenticationFilterExt
where
    Self: Send + Sync,
{
    async fn on_successful_authentication(
        &self,
        request: &dyn HttpRequest,
        response: &dyn HttpResponse,
        auth_result: &dyn Authentication,
    ) -> Result<(), BoxError>;

    async fn on_unsuccessful_authentication(
        &self,
        request: &dyn HttpRequest,
        response: &dyn HttpResponse,
        failed: &AuthenticationError,
    ) -> Result<(), BoxError>;
}
