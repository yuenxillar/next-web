#[cfg(feature = "web")]
use next_web_core::http::{HttpRequest, HttpResponse, HttpRequestShare, HttpResponseShare};
use next_web_core::{
    async_trait,
    error::{BoxError, IllegalStateError},
    Required,
};
use tracing::{debug, warn};

use crate::core::{
    authc::{authentication_token::AuthenticationToken, AuthenticationError},
    authz::authorization_error::AuthorizationError,
    session::{
        mgt::{
            default_session_context::DefaultSessionContext, session_context::SessionContext,
        },
        proxied_session::ProxiedSession,
        Session, SessionError, SessionValue,
    },
    subject::{principal_collection::PrincipalCollection, Subject},
    util::object::Object,
};
#[cfg(not(feature = "web"))]
use crate::core::mgt::security_manager::SecurityManager;
#[cfg(feature = "web")]
use crate::web::mgt::web_security_manager::WebSecurityManager;

#[cfg(feature = "web")]
use std::sync::Mutex;

use std::{
    any::Any,
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Clone)]
pub struct DelegatingSubject {
    principals: Option<Arc<dyn PrincipalCollection>>,
    authenticated: bool,
    host: Option<String>,
    pub(crate) session: Option<Arc<dyn Session>>,
    session_creation_enabled: bool,
    /// The request of the call the subject serves, when it was given one.
    #[cfg(feature = "web")]
    request_share: Option<Arc<HttpRequestShare>>,
    /// The response of the call the subject serves, when it was given one.
    #[cfg(feature = "web")]
    response_share: Option<Arc<Mutex<HttpResponseShare>>>,

    #[cfg(not(feature = "web"))]
    pub(crate) security_manager: Arc<dyn SecurityManager>,

    #[cfg(feature = "web")]
    pub(crate) security_manager: Arc<dyn WebSecurityManager>,
}

impl DelegatingSubject {
    const RUN_AS_PRINCIPALS_SESSION_KEY: &str = stringify!(format!(
        "{}{}",
        std::any::type_name::<Self>(),
        ".RUN_AS_PRINCIPALS_SESSION_KEY"
    ));

    /// Returns the request of the call the subject serves, when it was given one.
    #[cfg(feature = "web")]
    pub fn request_share(&self) -> Option<&Arc<HttpRequestShare>> {
        self.request_share.as_ref()
    }

    /// Returns the response of the call the subject serves, when it was given one.
    #[cfg(feature = "web")]
    pub fn response_share(&self) -> Option<&Arc<Mutex<HttpResponseShare>>> {
        self.response_share.as_ref()
    }

    /// Gives the subject the request and the response of its call.
    ///
    /// # Arguments
    ///
    /// * `request` - The snapshot of the request of the call.
    /// * `response` - The response of the call, shared with the filter chain.
    #[cfg(feature = "web")]
    pub fn attach_shared_context(
        &mut self,
        request: Arc<HttpRequestShare>,
        response: Arc<Mutex<HttpResponseShare>>,
    ) {
        self.request_share = Some(request);
        self.response_share = Some(response);
    }

    /// Writes what a response of the subject holds into the response of the call.
    ///
    /// # Arguments
    ///
    /// * `response` - The response a manager filled while it worked for the subject.
    #[cfg(feature = "web")]
    fn merge_response_share(&self, response: &HttpResponseShare) {
        let Some(target) = self.response_share.as_ref() else {
            return;
        };
        let Ok(mut target) = target.lock() else {
            return;
        };

        // Every header the manager added 鈥?the session id cookie above all 鈥?is
        // appended to the response of the call.
        for (name, value) in response.header_map().iter() {
            if let Ok(value) = value.to_str() {
                target.append_header(name.as_str(), value);
            }
        }
    }

    #[cfg(feature = "web")]
    pub fn new(
        principals: Option<Arc<dyn PrincipalCollection>>,
        authenticated: bool,
        host: Option<String>,
        session: Option<Arc<dyn Session>>,
        session_creation_enabled: bool,
        security_manager: Arc<dyn WebSecurityManager>,
    ) -> Self {
        let mut subject = Self {
            principals,
            authenticated,
            host,
            session_creation_enabled,
            session: None,
            request_share: None,
            response_share: None,
            security_manager,
        };

        let session = match session {
            Some(session) => subject.decorate(session),
            None => return subject,
        };

        subject.session = Some(session);
        subject
    }

    #[cfg(not(feature = "web"))]
    pub fn new(
        principals: Option<Arc<dyn PrincipalCollection>>,
        authenticated: bool,
        host: Option<String>,
        session: Option<Arc<dyn Session>>,
        session_creation_enabled: bool,
        security_manager: Arc<dyn SecurityManager>,
    ) -> Self {
        let mut subject = Self {
            principals,
            authenticated,
            host,
            session_creation_enabled,
            session: None,
            security_manager,
        };

        let session = match session {
            Some(session) => subject.decorate(session),
            None => return subject,
        };

        subject.session = Some(session);
        subject
    }

    pub fn decorate(&self, session: Arc<dyn Session>) -> Arc<dyn Session> {
        let session = StoppingAwareProxiedSession::new(session);
        Arc::new(session)
    }

    #[cfg(feature = "web")]
    pub fn get_security_manager(&self) -> &dyn WebSecurityManager {
        self.security_manager.as_ref()
    }

    #[cfg(not(feature = "web"))]
    pub fn get_security_manager(&self) -> &dyn SecurityManager {
        self.security_manager.as_ref()
    }

    /// Returns the manager the subject works through.
    ///
    /// The subject a login builds works through the manager of the subject it
    /// was built from, so the manager is handed over as the shared value both
    /// of them hold.
    #[cfg(feature = "web")]
    pub fn get_owned_security_manager(&self) -> Arc<dyn WebSecurityManager> {
        self.security_manager.clone()
    }

    #[cfg(not(feature = "web"))]
    pub fn get_owned_security_manager(&self) -> Arc<dyn SecurityManager> {
        self.security_manager.clone()
    }

    pub fn has_principals(&self) -> bool {
        match self.principals.as_ref() {
            Some(principals) => !principals.is_empty(),
            None => false,
        }
    }

    pub fn get_host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    pub fn get_primary_principal<'a>(
        &'a self,
        principals: Option<&'a dyn PrincipalCollection>,
    ) -> Option<&'a Object> {
        if let Some(principals) = principals {
            if !principals.is_empty() {
                return principals.get_primary_principal();
            }
        }

        None
    }

    pub fn is_session_creation_enabled(&self) -> bool {
        self.session_creation_enabled
    }

    pub fn session_stopped(&mut self) {
        self.session = None;
    }

    async fn get_run_as_principals_stack(&self) -> Option<Vec<Arc<dyn PrincipalCollection>>> {
        let session = self.get_session();

        let session = match session {
            Some(session) => session,
            None => return None,
        };

        if let Some(session_value) = session
            .get_attribute(Self::RUN_AS_PRINCIPALS_SESSION_KEY)
            .await
        {
            if let SessionValue::Object(obj) = session_value {
                if let Ok(principal_collections) = obj
                    .clone()
                    .into_any()
                    .downcast::<Vec<Arc<dyn PrincipalCollection>>>()
                {
                    return Some(*principal_collections);
                }
            }
        }

        None
    }

    pub fn assert_authz_check_possible(&self) -> Result<(), AuthorizationError> {
        if !self.has_principals() {
            let msg = format!("This subject is anonymous - it does not have any identifying principals and
                    authorization operations require an identity to check against.  A Subject instance will
                    acquire these identifying principals automatically after a successful login is performed
                    be executing {}.login(AuthenticationToken) or when 'Remember Me'
                    functionality is enabled by the SecurityManager.  This exception can also occur when a
                    previously logged-in Subject has logged out which
                    makes it anonymous again.  Because an identity is currently not known due to any of these
                    conditions, authorization is denied.", std::any::type_name::<dyn Subject>());

            return Err(AuthorizationError::Unauthorized(msg));
        }

        Ok(())
    }

    async fn push_identity(
        &mut self,
        principals: &Arc<dyn PrincipalCollection>,
    ) -> Result<(), IllegalStateError> {
        if principals.is_empty() {
            return Err(IllegalStateError { msg: "Specified Subject principals cannot be null or empty for 'run as' functionality.".to_string() });
        }

        let stack = self.get_run_as_principals_stack().await;
        let mut stack = match stack {
            Some(stack) => stack,
            None => Default::default(),
        };

        stack.insert(0, principals.clone());
        let session = self.get_session_or_create(false).await;

        if let Some(session) = session {
            session
                .set_attribute(
                    Self::RUN_AS_PRINCIPALS_SESSION_KEY,
                    SessionValue::Object(Box::new(stack)),
                )
                .await
                .unwrap();
        }
        Ok(())
    }

    async fn pop_identity(&mut self) -> Option<&dyn PrincipalCollection> {
        let popped = None;

        let stack = self.get_run_as_principals_stack().await;

        let mut stack = match stack {
            Some(stack) => stack,
            None => return None,
        };
        if !stack.is_empty() {
            stack.remove(0);

            if !stack.is_empty() {
                let session = self.get_session_or_create(false).await;
                if let Some(session) = session {
                    session
                        .set_attribute(
                            Self::RUN_AS_PRINCIPALS_SESSION_KEY,
                            SessionValue::Object(Box::new(stack)),
                        )
                        .await
                        .unwrap();
                }
            } else {
                self.clear_run_as_identities().await.unwrap();
            }
        }
        popped
    }

    pub(crate) async fn _logout(&mut self) -> Result<(), BoxError> {
        self.session = None;
        self.principals = None;
        self.authenticated = false;

        Ok(())
    }

    pub(crate) async fn _login(
        &mut self,
        mut subject: Box<dyn Subject>,
    ) -> Result<(), AuthenticationError> {
        self.clear_run_as_identities_internal().await;

        #[allow(unused_assignments)]
        let mut principals: Option<Arc<dyn PrincipalCollection>> = None;
        let mut host: Option<&str> = None;

        // let principal_collection: Option<Arc<dyn PrincipalCollection>> =
        let any: &dyn Any = &(subject.clone());

        if let Some(delegating) = any.downcast_ref::<Self>() {
            principals = delegating.principals.clone();
            host = delegating.host.as_deref();
        } else {
            principals = subject.get_principals().await.map(|pr| pr.clone());
        }

        if match principals.as_ref() {
            Some(val) => val.is_empty(),
            None => true,
        } {
            return Err(AuthenticationError::Custom(
                "Principals returned from security_manager.login( token ) returned a null or
            empty value.  This value must be non null and populated with one or more elements."
                    .to_string(),
            ));
        }

        self.principals = principals;
        self.authenticated = true;

        // if let Some(token) = (token as &dyn Any).downcast_ref::<HostAuthenticationToken>() {
        //     host = token.get_host();
        // }

        self.host = host.map(|s| s.to_string());

        let session = subject.get_session_or_create(false).await;

        match session {
            Some(session) => {
                self.session = Some(self.decorate(session));
            }
            None => {
                self.session = None;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Subject for DelegatingSubject {
    async fn get_principal(&self) -> Option<&Object> {
        self.get_primary_principal(self.get_principals().await.map(|pr| pr.as_ref()))
    }

    async fn get_principals(&self) -> Option<&Arc<dyn PrincipalCollection>> {
        // A subject that is acting as another identity answers with that identity,
        // and a subject that is not answers with the principals it was given. What
        // it must never answer is nothing: every authorization check of the subject
        // starts from the principals of the subject.
        // The principals of a `run as` identity live in the session, and the stack
        // is read out of it as an owned value: this signature answers with the
        // principals the subject was given. Reading the stack into the subject is
        // what a full `run as` support would need.
        let _ = self.get_run_as_principals_stack().await;

        self.principals.as_ref()
    }

    async fn is_permitted(&self, permission: &str) -> bool {
        self.has_principals()
            && self.security_manager.is_permitted_from_str(
                self.get_principals().await.map(|pr| pr.as_ref()),
                permission,
            )
    }

    async fn is_authenticated(&self) -> bool {
        self.authenticated && self.has_principals()
    }

    async fn is_remembered(&self) -> bool {
        if let Some(val) = self.get_principals().await {
            return !val.is_empty() && !self.is_authenticated().await;
        }
        false
    }

    async fn is_permitted_all(&self, permissions: &[&str]) -> bool {
        self.has_principals()
            && self.security_manager.is_permitted_all_from_str(
                self.get_principals().await.map(|pr| pr.as_ref()),
                permissions,
            )
    }

    async fn check_permission(&self, permission: &str) -> Result<(), AuthorizationError> {
        self.assert_authz_check_possible()?;
        self.security_manager.check_permission_from_str(
            self.get_principals().await.map(|pr| pr.as_ref()),
            permission,
        )
    }

    async fn check_permissions(&self, permissions: &[&str]) -> Result<(), AuthorizationError> {
        self.assert_authz_check_possible()?;
        self.security_manager.check_permissions_from_str(
            self.get_principals().await.map(|pr| pr.as_ref()),
            permissions,
        )
    }

    async fn has_role(&self, role_identifier: &str) -> bool {
        self.has_principals()
            && self.security_manager.has_role(
                self.get_principals().await.map(|pr| pr.as_ref()),
                role_identifier,
            )
    }

    async fn has_all_roles(&self, roles: &[&str]) -> bool {
        self.has_principals()
            && self
                .security_manager
                .has_all_roles(self.get_principals().await.map(|pr| pr.as_ref()), roles)
    }

    async fn check_role(&self, role: &str) -> Result<(), AuthorizationError> {
        self.assert_authz_check_possible()?;
        self.security_manager
            .check_role(self.get_principals().await.map(|pr| pr.as_ref()), role)
    }

    async fn check_roles(&self, roles: &[&str]) -> Result<(), AuthorizationError> {
        self.assert_authz_check_possible()?;
        self.security_manager
            .check_roles(self.get_principals().await.map(|pr| pr.as_ref()), roles)
    }

    fn get_session(&self) -> Option<&Arc<dyn Session>> {
        self.session.as_ref()
    }

    async fn get_session_or_create(&mut self, create: bool) -> Option<Arc<dyn Session>> {
        if self.session.is_none() && create {
            if !self.is_session_creation_enabled() {
                // String msg = "Session creation has been disabled for the current subject.  This exception indicates "
                // + "that there is either a programming error (using a session when it should never be "
                // + "used) or that Shiro's configuration needs to be adjusted to allow Sessions to be created "
                // + "for the current Subject.  See the " + DisabledSessionException.class.getName() + " JavaDoc "
                // + "for more.";
                return None;
            }

            let session_context = self.create_session_context();

            // A subject that was given the request of its call asks the manager for
            // a session with that request: the web manager writes the session id
            // cookie into the response while it starts the session, which a manager
            // called without a request cannot do.
            #[cfg(feature = "web")]
            let created = match self.request_share.clone() {
                Some(request) => {
                    let mut request = (*request).clone();
                    let mut response = HttpResponseShare::default();

                    match self
                        .security_manager
                        .start(&session_context, &mut request, &mut response)
                        .await
                    {
                        Ok(session) => {
                            self.merge_response_share(&response);

                            Ok(Arc::from(session))
                        }
                        Err(error) => Err(error.to_string()),
                    }
                }
                None => self
                    .security_manager
                    .start_session(&session_context)
                    .await
                    .map_err(|error| error.to_string()),
            };

            #[cfg(not(feature = "web"))]
            let created = self
                .security_manager
                .start_session(&session_context)
                .await
                .map_err(|error| error.to_string());

            match created {
                Ok(session) => self.session = Some(session),
                Err(error) => {
                    warn!("Could not create a session for the subject: {error}");
                    return None;
                }
            }
        }

        self.session.clone()
    }

    #[allow(unused_variables)]
    async fn login(
        &mut self,
        token: &dyn AuthenticationToken,
        #[cfg(feature = "web")] req: &mut dyn HttpRequest,
        #[cfg(feature = "web")] resp: &mut dyn HttpResponse,
    ) -> Result<(), AuthenticationError> {
        #[cfg(not(feature = "web"))]
        {
            let subject = self.security_manager.login(&*self, token).await?;

            return self._login(subject).await;
        }

        #[cfg(feature = "web")]
        {
            Ok(())
        }
    }

    async fn logout(
        &mut self,
        #[cfg(feature = "web")] _req: &mut dyn HttpRequest,
        #[cfg(feature = "web")] _resp: &mut dyn HttpResponse,
    ) -> Result<(), BoxError> {
        #[cfg(not(feature = "web"))]
        {
            self.clear_run_as_identities_internal().await;
            if let Err(error) = self.security_manager.logout(&*self).await {
                debug!(
                    "Encountered session exception trying to log out subject.  This can generally safely be ignored. {:?}",
                    error
                )
            }

            return self._logout().await;
        }

        #[cfg(feature = "web")]
        {
            Ok(())
        }
    }

    // ======================================
    // 'Run As' support implementations
    // ======================================
    async fn run_as(
        &mut self,
        principals: &Arc<dyn PrincipalCollection>,
    ) -> Result<(), IllegalStateError> {
        if !self.has_principals() {
            let msg = format!(
                "This subject does not yet have an identity.  Assuming the identity of another
                Subject is only allowed for Subjects with an existing identity.  Try logging this subject in
                first, or using the {} to build ad hoc Subject instances with identities as necessary.",
                std::any::type_name::<dyn Subject>(),
            );

            return Err(IllegalStateError { msg });
        }

        self.push_identity(principals).await
    }

    async fn is_run_as(&self) -> bool {
        // A subject acts as another identity only when it holds the stack of
        // those identities; a subject that was never given one is not `run as`.
        self.get_run_as_principals_stack()
            .await
            .map(|stack| !stack.is_empty())
            .unwrap_or_default()
    }

    async fn get_previous_principals(&self) -> Option<Arc<dyn PrincipalCollection>> {
        let mut previous_principals = None;
        let stack = self.get_run_as_principals_stack().await;

        let mut stack = match stack {
            Some(stack) => stack,
            None => return previous_principals,
        };

        let statck_size = stack.len();
        if statck_size > 0 {
            if statck_size == 1 {
                previous_principals = self.principals.clone();
            } else {
                previous_principals = Some(stack.remove(1));
            }
        }

        previous_principals
    }

    async fn release_run_as(&mut self) -> Option<&dyn PrincipalCollection> {
        self.pop_identity().await
    }
}

impl DelegatingSubject {
    /// Returns the context a session of this subject is created with.
    ///
    /// A subject of the core tree opens its session from this context alone; a
    /// web subject gives the session the request of its call as well, so the
    /// manager can write the session id cookie into the response.
    fn create_session_context(&self) -> impl SessionContext + 'static {
        let mut session_context = DefaultSessionContext::default();
        if let Some(host) = self.host.as_ref() {
            if host.trim().len() > 0 {
                session_context.set_host(host.as_str());
            }
        }

        session_context
    }

    pub(crate) async fn clear_run_as_identities_internal(&mut self) {
        if let Err(se) = self.clear_run_as_identities().await {
            debug!(
                "Encountered session exception trying to clear 'runAs' identities during logout.  This can generally safely be ignored. {:?}",
                se
            )
        }
    }

    async fn clear_run_as_identities(&mut self) -> Result<Option<SessionValue>, SessionError> {
        let session = self.get_session_or_create(false).await;
        match session {
            Some(session) => {
                session
                    .remove_attribute(Self::RUN_AS_PRINCIPALS_SESSION_KEY)
                    .await
            }
            None => Err(SessionError::Invalid(None)),
        }
    }
}

#[derive(Clone)]
pub struct StoppingAwareProxiedSession
where
    Self: Required<ProxiedSession>,
{
    // owner: &'a mut DelegatingSubject,
    proxied_session: ProxiedSession,
}

impl StoppingAwareProxiedSession {
    fn new(target: Arc<dyn Session>) -> Self {
        let proxied_session = ProxiedSession::new(target);

        Self { proxied_session }
    }

    pub async fn stop(&mut self, subject: &mut DelegatingSubject) {
        self.proxied_session.stop().await.ok();
        subject.session_stopped();
    }
}

#[async_trait]
impl Session for StoppingAwareProxiedSession {
    fn id(&self) -> &crate::core::session::SessionId {
        self.proxied_session.id()
    }

    async fn start_timestamp(&self) -> i64 {
        self.proxied_session.start_timestamp().await
    }

    async fn last_access_time(&self) -> Option<i64> {
        self.proxied_session.last_access_time().await
    }

    async fn timeout(&self) -> Result<i64, SessionError> {
        self.proxied_session.timeout().await
    }

    async fn set_timeout(&self, max_idle_time_in_millis: i64) -> Result<(), SessionError> {
        self.proxied_session.set_timeout(max_idle_time_in_millis).await
    }

    async fn host(&self) -> Option<String> {
        self.proxied_session.host().await
    }

    async fn touch(&self) -> Result<(), SessionError> {
        self.proxied_session.touch().await
    }

    async fn stop(&self) -> Result<(), SessionError> {
        self.proxied_session.stop().await
    }

    async fn attribute_keys(&self) -> Result<Vec<String>, SessionError> {
        self.proxied_session.attribute_keys().await
    }

    async fn get_attribute(&self, key: &str) -> Option<SessionValue> {
        self.proxied_session.get_attribute(key).await
    }

    async fn set_attribute(&self, key: &str, value: SessionValue) -> Result<(), SessionError> {
        self.proxied_session.set_attribute(key, value).await
    }

    async fn remove_attribute(&self, key: &str) -> Result<Option<SessionValue>, SessionError> {
        self.proxied_session.remove_attribute(key).await
    }
}

impl Required<ProxiedSession> for StoppingAwareProxiedSession {
    fn get_object(&self) -> &ProxiedSession {
        &self.proxied_session
    }

    fn get_mut_object(&mut self) -> &mut ProxiedSession {
        &mut self.proxied_session
    }
}

impl Deref for StoppingAwareProxiedSession {
    type Target = ProxiedSession;

    fn deref(&self) -> &Self::Target {
        self.get_object()
    }
}

impl DerefMut for StoppingAwareProxiedSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut_object()
    }
}
pub trait DelegatingSubjectSupport
where
    Self: Send + Sync,
    Self: Subject,
{
    fn is_session_creation_enabled(&self) -> bool;
}

impl Display for DelegatingSubject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DelegatingSubject {{ {}  authenticated={}, host='******', session='******', {} {} }}",
            format_args!(
                "principals={:?}, ",
                self.principals.as_ref().map(|x| x.to_string())
            ),
            self.authenticated,
            format_args!(
                "session_creation_enabled={}, ",
                self.session_creation_enabled
            ),
            format_args!("security_manager={}, ", self.security_manager)
        )
    }
}
