use std::sync::Arc;

use next_web_core::{
    async_trait,
    filter::FilterError,
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
    util::http_method::HttpMethod,
};
use tracing::debug;

use crate::{
    authentication::AccountStatusUserDetailsChecker,
    core::{
        authentication_error::{AuthenticationError, AuthenticationErrorKind},
        context::{security_context_holder::SecurityContextHolder, SecurityContextHolderStrategy},
        granted_authority::GrantedAuthority,
        simple_granted_authority::SimpleGrantedAuthority,
        userdetails::{UserDetails, UserDetailsChecker, UserDetailsService},
        username_password_authentication_token::UsernamePasswordAuthenticationToken,
    },
    web::{
        authentication::{
            authentication_failure_handler::AuthenticationFailureHandler,
            authentication_success_handler::AuthenticationSuccessHandler,
            simple_url_authentication_failure_handler::SimpleUrlAuthenticationFailureHandler,
            simple_url_authentication_success_handler::SimpleUrlAuthenticationSuccessHandler,
            switchuser::{
                switch_user_authority_changer::SwitchUserAuthorityChanger,
                switch_user_granted_authority::SwitchUserGrantedAuthority,
            },
        },
        context::SecurityContextRepository,
        util::matcher::{PathPatternRequestMatcher, RequestMatcher},
    },
};

/// Switch User processing filter responsible for user context switching.
///
/// This filter is similar to Unix 'su' however for Spring Security-managed web
/// applications. A common use-case for this feature is the ability to allow
/// higher-authority users (e.g. `ROLE_ADMIN`) to switch to a regular user
/// (e.g. `ROLE_USER`).
///
/// This filter assumes that the user performing the switch will be required to
/// be logged in as normal (i.e. as a `ROLE_ADMIN` user). The user will then
/// access a page/controller that enables the administrator to specify who they
/// wish to become (see `switchUserUrl`).
///
/// **Note: This URL will be required to have appropriate security constraints
/// configured so that only users of that role can access it (e.g. `ROLE_ADMIN`).**
///
/// On a successful switch, the user's `SecurityContext` will be updated to
/// reflect the specified user and will also contain an additional
/// `SwitchUserGrantedAuthority` which contains the original user. Before
/// switching, a check will be made on whether the user is already currently
/// switched, and any current switch will be exited to prevent "nested" switches.
///
/// To 'exit' from a user context, the user needs to access a URL (see
/// `exitUserUrl`) that will switch back to the original user as identified by
/// the `ROLE_PREVIOUS_ADMINISTRATOR`.
#[derive(Clone)]
pub struct SwitchUserFilter {
    /// The security context holder strategy to use.
    security_context_holder_strategy: Arc<dyn SecurityContextHolderStrategy>,

    /// Matches requests to `/logout/impersonate` to signal an exit from
    /// the switched user context.
    exit_user_matcher: Arc<dyn RequestMatcher>,

    /// Matches requests to `/login/impersonate` to signal a switch to
    /// another user.
    switch_user_matcher: Arc<dyn RequestMatcher>,

    /// The target URL to redirect to after a successful switch or exit.
    target_url: Option<String>,

    /// The URL to redirect to if the switch fails.
    switch_failure_url: Option<String>,

    /// The parameter name containing the target username. Defaults to `"username"`.
    username_parameter: String,

    /// The authority role name used to identify the previous administrator.
    /// Defaults to `"ROLE_PREVIOUS_ADMINISTRATOR"`.
    switch_authority_role: String,

    /// Optional strategy to fine-tune the authorities granted to the target user.
    switch_user_authority_changer: Option<Arc<dyn SwitchUserAuthorityChanger>>,

    /// The service used to load the target user's details.
    user_details_service: Arc<dyn UserDetailsService>,

    /// The checker used to validate the target user's account status.
    user_details_checker: Arc<dyn UserDetailsChecker>,

    /// Called on successful switch or exit.
    success_handler: Arc<dyn AuthenticationSuccessHandler>,

    /// Called when a switch attempt fails.
    failure_handler: Arc<dyn AuthenticationFailureHandler>,

    /// The security context repository used to persist the security context
    /// on switch success. If `None`, the context is only stored on the
    /// `SecurityContextHolderStrategy` for the duration of the request.
    security_context_repository: Option<Arc<dyn SecurityContextRepository>>,
}

impl SwitchUserFilter {
    /// The default parameter name for the target username.
    pub const SPRING_SECURITY_SWITCH_USERNAME_KEY: &'static str = "username";

    /// The role identifier for the previous administrator.
    pub const ROLE_PREVIOUS_ADMINISTRATOR: &'static str = "ROLE_PREVIOUS_ADMINISTRATOR";

    /// Creates a new `SwitchUserFilter` with a default target URL of `"/"`.
    ///
    /// # Parameters
    /// * `user_details_service` - The service used to load the target user to
    ///   switch to. Cannot be null.
    pub fn new(user_details_service: Arc<dyn UserDetailsService>) -> Self {
        let target_url = "/".to_string();
        Self {
            security_context_holder_strategy: SecurityContextHolder::get_context_holder_strategy(),
            exit_user_matcher: Self::create_matcher("/logout/impersonate"),
            switch_user_matcher: Self::create_matcher("/login/impersonate"),
            target_url: Some(target_url.clone()),
            switch_failure_url: None,
            username_parameter: Self::SPRING_SECURITY_SWITCH_USERNAME_KEY.to_string(),
            switch_authority_role: Self::ROLE_PREVIOUS_ADMINISTRATOR.to_string(),
            switch_user_authority_changer: None,
            user_details_service,
            user_details_checker: Arc::new(AccountStatusUserDetailsChecker::default()),
            success_handler: Arc::new(SimpleUrlAuthenticationSuccessHandler::new(&target_url)),
            failure_handler: Arc::new(SimpleUrlAuthenticationFailureHandler::new("/login")),
            security_context_repository: None,
        }
    }

    /// Creates a request matcher for the given URL pattern using `POST` method.
    fn create_matcher(pattern: &str) -> Arc<dyn RequestMatcher> {
        Arc::new(PathPatternRequestMatcher::path_pattern(
            Some(HttpMethod::Post),
            pattern,
        ))
    }

    // --- Setters ---

    /// Sets the security context holder strategy.
    pub fn set_security_context_holder_strategy(
        &mut self,
        strategy: Arc<dyn SecurityContextHolderStrategy>,
    ) {
        self.security_context_holder_strategy = strategy;
    }

    /// Sets the URL to respond to exit user processing.
    /// This is a shortcut for `set_exit_user_matcher`.
    pub fn set_exit_user_url(&mut self, exit_user_url: &str) {
        assert!(
            !exit_user_url.is_empty() && exit_user_url.starts_with('/'),
            "exitUserUrl cannot be empty and must be a valid redirect URL"
        );
        self.exit_user_matcher = Self::create_matcher(exit_user_url);
    }

    /// Sets the matcher to respond to exit user processing.
    pub fn set_exit_user_matcher(&mut self, exit_user_matcher: Arc<dyn RequestMatcher>) {
        self.exit_user_matcher = exit_user_matcher;
    }

    /// Sets the URL to respond to switch user processing.
    /// This is a shortcut for `set_switch_user_matcher`.
    pub fn set_switch_user_url(&mut self, switch_user_url: &str) {
        assert!(
            !switch_user_url.is_empty() && switch_user_url.starts_with('/'),
            "switchUserUrl cannot be empty and must be a valid redirect URL"
        );
        self.switch_user_matcher = Self::create_matcher(switch_user_url);
    }

    /// Sets the matcher to respond to switch user processing.
    pub fn set_switch_user_matcher(&mut self, switch_user_matcher: Arc<dyn RequestMatcher>) {
        self.switch_user_matcher = switch_user_matcher;
    }

    /// Sets the URL to go to after a successful switch / exit user request.
    /// Use `set_success_handler` instead if you need more customized behaviour.
    pub fn set_target_url(&mut self, target_url: String) {
        self.target_url = Some(target_url.clone());
        self.success_handler = Arc::new(SimpleUrlAuthenticationSuccessHandler::new(&target_url));
    }

    /// Used to define custom behaviour on a successful switch or exit user.
    /// Can be used instead of setting `target_url`.
    pub fn set_success_handler(&mut self, success_handler: Arc<dyn AuthenticationSuccessHandler>) {
        self.success_handler = success_handler;
    }

    /// Sets the URL to which a user should be redirected if the switch fails.
    /// If not set, an error message will be written to the response.
    /// Use `set_failure_handler` instead if you need more customized behaviour.
    pub fn set_switch_failure_url(&mut self, switch_failure_url: String) {
        assert!(
            !switch_failure_url.is_empty() && switch_failure_url.starts_with('/'),
            "switchFailureUrl must be a valid redirect URL"
        );
        self.switch_failure_url = Some(switch_failure_url.clone());
        self.failure_handler = Arc::new(SimpleUrlAuthenticationFailureHandler::new(
            &switch_failure_url,
        ));
    }

    /// Used to define custom behaviour when a switch fails.
    /// Can be used instead of setting `switch_failure_url`.
    pub fn set_failure_handler(&mut self, failure_handler: Arc<dyn AuthenticationFailureHandler>) {
        self.failure_handler = failure_handler;
    }

    /// Sets the `SwitchUserAuthorityChanger` to fine-tune the authorities
    /// granted to the target user during a switch.
    pub fn set_switch_user_authority_changer(
        &mut self,
        changer: Arc<dyn SwitchUserAuthorityChanger>,
    ) {
        self.switch_user_authority_changer = Some(changer);
    }

    /// Sets the `UserDetailsChecker` that is called on the target user whenever
    /// the user is switched. Defaults to `AccountStatusUserDetailsChecker`.
    pub fn set_user_details_checker(&mut self, checker: Arc<dyn UserDetailsChecker>) {
        self.user_details_checker = checker;
    }

    /// Allows the parameter containing the username to be customized.
    /// Defaults to `"username"`.
    pub fn set_username_parameter(&mut self, username_parameter: String) {
        self.username_parameter = username_parameter;
    }

    /// Allows the role of the switch authority to be customized.
    /// Defaults to `"ROLE_PREVIOUS_ADMINISTRATOR"`.
    pub fn set_switch_authority_role(&mut self, switch_authority_role: String) {
        assert!(
            !switch_authority_role.is_empty(),
            "switchAuthorityRole cannot be null"
        );
        self.switch_authority_role = switch_authority_role;
    }

    /// Sets the `SecurityContextRepository` to save the `SecurityContext` on
    /// switch user success. The default is `None` (context not persisted).
    pub fn set_security_context_repository(&mut self, repo: Arc<dyn SecurityContextRepository>) {
        self.security_context_repository = Some(repo);
    }

    // --- Filter logic ---

    /// Checks the request URI for the presence of `exitUserUrl`.
    fn requires_exit_user(&self, request: &dyn HttpRequest) -> bool {
        self.exit_user_matcher.matches(request)
    }

    /// Checks the request URI for the presence of `switchUserUrl`.
    fn requires_switch_user(&self, request: &dyn HttpRequest) -> bool {
        self.switch_user_matcher.matches(request)
    }

    /// Find the original `Authentication` object from the current user's
    /// granted authorities. A successfully switched user should have a
    /// `SwitchUserGrantedAuthority` that contains the original source user
    /// `Authentication` object.
    fn get_source_authentication(
        &self,
        current: &dyn crate::core::Authentication,
    ) -> Option<Arc<dyn crate::core::Authentication>> {
        // Downcast to UsernamePasswordAuthenticationToken to access the
        // underlying authority objects.
        // let upat = current
        //     .as_any()
        //     .downcast_ref::<UsernamePasswordAuthenticationToken>()?;
        // for authority in upat.authorities_objects() {
        //     // if let Some(source) = authority.as_switch_user_source() {
        //     //     debug!(
        //     //         "Found original switch user granted authority [{:?}]",
        //     //         source.get_name()
        //     //     );
        //     //     return Some(source);
        //     // }
        // }

        todo!()
    }

    /// Retrieves the current `Authentication`, attempting to find the original
    /// (pre-switch) authentication first. If the user is already switched, the
    /// original is returned; otherwise the current context authentication is used.
    fn get_current_authentication(&self) -> Option<Arc<dyn crate::core::Authentication>> {
        match self.attempt_exit_user() {
            Ok(original) => Some(original),
            Err(_) => self
                .security_context_holder_strategy
                .get_context()
                .and_then(|ctx| ctx.get_authentication().cloned()),
        }
    }

    /// Create a switch user token that contains an additional `GrantedAuthority`
    /// that contains the original `Authentication` object.
    async fn create_switch_user_token(
        &self,
        target_user: &dyn UserDetails,
    ) -> UsernamePasswordAuthenticationToken {
        // Grant an additional authority that contains the original Authentication
        // object which will be used to 'exit' from the current switched user.
        let current_authentication = self
            .get_current_authentication()
            .expect("currentAuthentication cannot be null");

        // Clone the Arc before moving it into SwitchUserGrantedAuthority,
        // since we may need it again for the authority changer.
        let switch_authority: Arc<dyn GrantedAuthority> =
            Arc::new(SwitchUserGrantedAuthority::new(
                &self.switch_authority_role,
                current_authentication.clone(),
            ));

        // Get the original authorities from the target user and convert to owned arcs.
        let mut orig: Vec<Arc<dyn GrantedAuthority>> = Vec::new();
        for auth in target_user.authorities() {
            if let Some(name) = auth.authority() {
                orig.push(Arc::new(SimpleGrantedAuthority::new(name)));
            }
        }

        // Allow the changer to modify the authorities to be granted.
        if let Some(changer) = &self.switch_user_authority_changer {
            orig = changer.modify_granted_authorities(
                target_user,
                current_authentication.as_ref(),
                orig,
            );
        }

        // Add the new switch user authority.
        orig.push(switch_authority);

        // Create the new authentication token.
        UsernamePasswordAuthenticationToken::authenticated(
            target_user.username(),
            target_user.password().map(ToString::to_string),
            orig,
        )
    }

    /// Attempt to switch to another user. If the user does not exist or is not
    /// active, an error is returned.
    ///
    /// # Returns
    /// The new `Authentication` if successfully switched to another user.
    ///
    /// # Errors
    /// * `UsernameNotFoundError`   - If the target user is not found.
    /// * Locked account error      - If the account is locked.
    /// * Disabled account error    - If the target user is disabled.
    /// * Expired account error     - If the target user account is expired.
    /// * Expired credentials error - If the target user credentials are expired.
    async fn attempt_switch_user(
        &self,
        request: &mut dyn HttpRequest,
    ) -> Result<Arc<dyn crate::core::Authentication>, AuthenticationError> {
        // Extract username before any .await to avoid holding &dyn HttpRequest
        // across yield points (required for Send future).
        let username = request
            .parameter(&self.username_parameter)
            .unwrap_or("")
            .to_string();
        debug!("Attempting to switch to user [{}]", username);

        let target_user = self
            .user_details_service
            .load_user_by_username(&username)
            .await
            .map_err(|e| {
                AuthenticationError::with_kind(
                    format!("User not found: {}", e),
                    AuthenticationErrorKind::BadCredentials,
                )
            })?;

        self.user_details_checker
            .check(target_user.as_ref())
            .await?;

        // Create the switch user token.
        let token = self.create_switch_user_token(target_user.as_ref()).await;

        Ok(Arc::new(token))
    }

    /// Attempt to exit from an already switched user.
    ///
    /// # Returns
    /// The original `Authentication` object.
    ///
    /// # Errors
    /// * `AuthenticationCredentialsNotFound` - If no `Authentication` is
    ///   associated with this request or no original user is found.
    fn attempt_exit_user(
        &self,
    ) -> Result<Arc<dyn crate::core::Authentication>, AuthenticationError> {
        // Need to check to see if the current user has a SwitchUserGrantedAuthority.
        let current = self
            .security_context_holder_strategy
            .get_context()
            .and_then(|ctx| ctx.get_authentication().cloned())
            .ok_or_else(|| {
                AuthenticationError::with_kind(
                    "No current user associated with this request",
                    AuthenticationErrorKind::CredentialsNotFound,
                )
            })?;

        // Check to see if the current user did actually switch to another user.
        // If so, get the original source user so we can switch back.
        let original = self
            .get_source_authentication(current.as_ref())
            .ok_or_else(|| {
                debug!("Failed to find original user");
                AuthenticationError::with_kind(
                    "Failed to find original user",
                    AuthenticationErrorKind::CredentialsNotFound,
                )
            })?;

        Ok(original)
    }

    /// Updates the security context with the given authentication, persists it,
    /// and invokes the success handler.
    async fn successful_switch(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        authentication: Arc<dyn crate::core::Authentication>,
    ) -> Result<(), FilterError> {
        let context = self.security_context_holder_strategy.create_empty_context();
        context.set_authentication(Some(authentication.clone()));
        self.security_context_holder_strategy
            .set_context(context.clone());
        debug!("Set SecurityContextHolder to {}", authentication.get_name());
        if let Some(repo) = &self.security_context_repository {
            repo.save_context(&context, request, response).await;
        }
        self.success_handler
            .on_authentication_success(request, response, authentication.as_ref());
        Ok(())
    }
}

#[async_trait]
impl HttpFilter for SwitchUserFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        // Check for switch request.
        if self.requires_switch_user(request) {
            match self.attempt_switch_user(request).await {
                Ok(target_user) => {
                    self.successful_switch(request, response, target_user)
                        .await?;
                }
                Err(ex) => {
                    debug!("Failed to switch user: {:?}", ex);
                    self.failure_handler
                        .on_authentication_failure(request, response, &ex)
                        .map_err(FilterError::from)?;
                }
            }
            return Ok(());
        }

        // Check for exit request.
        if self.requires_exit_user(request) {
            match self.attempt_exit_user() {
                Ok(original_user) => {
                    self.successful_switch(request, response, original_user)
                        .await?;
                }
                Err(ex) => {
                    debug!("Failed to exit user: {:?}", ex);
                    self.failure_handler
                        .on_authentication_failure(request, response, &ex)
                        .map_err(FilterError::from)?;
                }
            }
            return Ok(());
        }

        debug!(
            "Did not attempt to switch user since request did not match [{:?}] or [{:?}]",
            self.switch_user_matcher, self.exit_user_matcher
        );
        filter_chain.do_filter(request, response).await
    }
}

impl Named for SwitchUserFilter {
    fn name(&self) -> &str {
        "SwitchUserFilter"
    }
}
