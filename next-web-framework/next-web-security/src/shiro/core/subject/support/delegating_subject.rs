use chrono::{DateTime, Utc};
use next_web_core::{
    clone_box,
    convert::into_box::IntoBox,
    error::{illegal_state_error::IllegalStateError, BoxError},
    traits::required::Required,
};
use tracing::debug;

use crate::core::{
    authc::{authentication_error::AuthenticationError, authentication_token::AuthenticationToken},
    authz::authorization_error::AuthorizationError,
    mgt::{default_security_manager::DefaultSecurityManager, security_manager::SecurityManager},
    session::{
        mgt::{default_session_context::DefaultSessionContext, session_context::SessionContext},
        proxied_session::ProxiedSession,
        Session, SessionError, SessionValue,
    },
    subject::{principal_collection::PrincipalCollection, Subject},
    util::object::Object,
};
use std::{
    any::Any,
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Clone)]
pub struct DelegatingSubject<T = DefaultSecurityManager> {
    principals: Option<Arc<dyn PrincipalCollection>>,
    authenticated: bool,
    host: Option<String>,
    pub(crate) session: Option<Box<dyn Session>>,
    session_creation_enabled: bool,
    security_manager: T,
}

impl<T> DelegatingSubject<T>
where
    T: SecurityManager,
    T: Clone + 'static,
{
    const RUN_AS_PRINCIPALS_SESSION_KEY: &str = stringify!(format!(
        "{}{}",
        std::any::type_name::<Self>(),
        ".RUN_AS_PRINCIPALS_SESSION_KEY"
    ));

    pub fn new(
        principals: Option<Arc<dyn PrincipalCollection>>,
        authenticated: bool,
        host: Option<String>,
        session: Option<Box<dyn Session>>,
        session_creation_enabled: bool,
        security_manager: T,
    ) -> Self {
        let mut subject = Self {
            principals,
            authenticated,
            host,
            session_creation_enabled,
            session: None,
            security_manager,
        };
        if session.is_some() {}
        let session = match session {
            Some(session) => subject.decorate(session),
            None => return subject,
        };

        subject.session = Some(session);
        subject
    }

    pub fn decorate(&self, session: Box<dyn Session>) -> Box<dyn Session> {
        let session = StoppingAwareProxiedSession::new(session);
        Box::new(session)
    }

    pub fn get_security_manager(&self) -> &T {
        &self.security_manager
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
        principals: Option<&dyn PrincipalCollection>,
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

    fn get_run_as_principals_stack(&self) -> Option<Vec<Arc<dyn PrincipalCollection>>> {
        let session = self.get_session();

        let session = match session {
            Some(session) => session,
            None => return None,
        };

        if let Some(session_value) = session.get_attribute(Self::RUN_AS_PRINCIPALS_SESSION_KEY) {
            if let SessionValue::Object(obj) = session_value {
                if let Ok(principal_collections) = obj
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

    fn push_identity(
        &mut self,
        principals: &Arc<dyn PrincipalCollection>,
    ) -> Result<(), IllegalStateError> {
        if principals.is_empty() {
            return Err(IllegalStateError { msg: "Specified Subject principals cannot be null or empty for 'run as' functionality.".to_string() });
        }

        let stack = self.get_run_as_principals_stack();
        let mut stack = match stack {
            Some(stack) => stack,
            None => Default::default(),
        };

        stack.insert(0, principals.clone());
        let session = self.get_session_or_create(false);

        if let Some(session) = session {
            session
                .set_attribute(
                    Self::RUN_AS_PRINCIPALS_SESSION_KEY,
                    SessionValue::Object(Box::new(stack)),
                )
                .unwrap();
        }
        Ok(())
    }

    fn pop_identity(&mut self) -> Option<&dyn PrincipalCollection> {
        let popped = None;

        let stack = self.get_run_as_principals_stack();

        let mut stack = match stack {
            Some(stack) => stack,
            None => return None,
        };
        if !stack.is_empty() {
            stack.remove(0);

            if !stack.is_empty() {
                let session = self.get_session_or_create(false);
                if let Some(session) = session {
                    session
                        .set_attribute(
                            Self::RUN_AS_PRINCIPALS_SESSION_KEY,
                            SessionValue::Object(stack.into_boxed()),
                        )
                        .unwrap();
                }
            } else {
                self.clear_run_as_identities().unwrap();
            }
        }
        popped
    }
}

impl<T> From<T> for DelegatingSubject<T>
where
    T: SecurityManager,
    T: Clone + 'static,
{
    fn from(security_manager: T) -> Self {
        Self::new(None, false, None, None, true, security_manager)
    }
}

impl<T> Subject for DelegatingSubject<T>
where
    T: SecurityManager + Clone,
    T: 'static,
{
    fn get_principal(&self) -> Option<&Object> {
        self.get_primary_principal(self.get_principals().map(|pr| pr.as_ref()))
    }

    fn get_principals(&self) -> Option<&Arc<dyn PrincipalCollection>> {
        let run_as_principals = self.get_run_as_principals_stack();
        match run_as_principals {
            Some(principals) => {
                if principals.is_empty() {
                    return self.principals.as_ref();
                }
                // else {
                // principals.get(0).map(|pr| pr.as_ref())
                // }
                None
            }
            None => None,
        }
    }

    fn is_permitted(&self, permission: &str) -> bool {
        self.has_principals()
            && self
                .security_manager
                .is_permitted_from_str(self.get_principals().map(|pr| pr.as_ref()), permission)
    }

    fn is_authenticated(&self) -> bool {
        self.authenticated && self.has_principals()
    }

    fn is_remembered(&self) -> bool {
        if let Some(val) = self.get_principals() {
            return !val.is_empty() && !self.is_authenticated();
        }
        false
    }

    fn is_permitted_all(&self, permissions: &[&str]) -> bool {
        self.has_principals()
            && self
                .security_manager
                .is_permitted_all_from_str(self.get_principals().map(|pr| pr.as_ref()), permissions)
    }

    fn check_permission(&self, permission: &str) -> Result<(), AuthorizationError> {
        self.assert_authz_check_possible()?;
        self.security_manager
            .check_permission_from_str(self.get_principals().map(|pr| pr.as_ref()), permission)
    }

    fn check_permissions(&self, permissions: &[&str]) -> Result<(), AuthorizationError> {
        self.assert_authz_check_possible()?;
        self.security_manager
            .check_permissions_from_str(self.get_principals().map(|pr| pr.as_ref()), permissions)
    }

    fn has_role(&self, role_identifier: &str) -> bool {
        self.has_principals()
            && self
                .security_manager
                .has_role(self.get_principals().map(|pr| pr.as_ref()), role_identifier)
    }

    fn has_all_roles(&self, roles: &[&str]) -> bool {
        self.has_principals()
            && self
                .security_manager
                .has_all_roles(self.get_principals().map(|pr| pr.as_ref()), roles)
    }

    fn check_role(&self, role: &str) -> Result<(), AuthorizationError> {
        self.assert_authz_check_possible()?;
        self.security_manager
            .check_role(self.get_principals().map(|pr| pr.as_ref()), role)
    }

    fn check_roles(&self, roles: &[&str]) -> Result<(), AuthorizationError> {
        self.assert_authz_check_possible()?;
        self.security_manager
            .check_roles(self.get_principals().map(|pr| pr.as_ref()), roles)
    }

    fn get_session(&self) -> Option<&dyn Session> {
        self.session.as_deref()
    }

    fn get_session_or_create(&mut self, create: bool) -> Option<&mut Box<dyn Session>> {
        if self.session.is_none() && create {
            if !self.is_session_creation_enabled() {
                // String msg = "Session creation has been disabled for the current subject.  This exception indicates "
                // + "that there is either a programming error (using a session when it should never be "
                // + "used) or that Shiro's configuration needs to be adjusted to allow Sessions to be created "
                // + "for the current Subject.  See the " + DisabledSessionException.class.getName() + " JavaDoc "
                // + "for more.";
                return None;
            }

            // TODO: create session
            // let session_context = self.create_session_context();
            // let session = self.security_manager.start(&session_context);
            // self.session = Some(self.decorate(session));
        }

        self.session.as_mut()
    }

    fn login(&mut self, token: &dyn AuthenticationToken) -> Result<(), AuthenticationError> {
        self.clear_run_as_identities_internal();

        let mut subject = self.security_manager.login(self, token)?;

        #[allow(unused_assignments)]
        let mut principals: Option<Arc<dyn PrincipalCollection>> = None;
        let mut host: Option<&str> = None;

        // let principal_collection: Option<Arc<dyn PrincipalCollection>> =
        let any: &dyn Any = &(subject.clone());

        if let Some(delegating) = any.downcast_ref::<Self>() {
            principals = delegating.principals.clone();
            host = delegating.host.as_deref();
        } else {
            principals = subject.get_principals().map(|pr| pr.clone());
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

        let session = subject.get_session_or_create(false);

        match session {
            Some(session) => {
                self.session = Some(self.decorate(clone_box(session.as_ref())));
            }
            None => {
                self.session = None;
            }
        }
        Ok(())
    }

    fn logout(&mut self) -> Result<(), BoxError> {
        self.clear_run_as_identities_internal();
        if let Err(error) = self.security_manager.logout(self) {
            debug!(
                "Encountered session exception trying to log out subject.  This can generally safely be ignored. {:?}",
                error
            )
        }

        self.session = None;
        self.principals = None;
        self.authenticated = false;

        Ok(())
    }

    // ======================================
    // 'Run As' support implementations
    // ======================================
    fn run_as(
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

        self.push_identity(principals)
    }

    fn is_run_as(&self) -> bool {
        !self
            .get_run_as_principals_stack()
            .map(|s| s.is_empty())
            .unwrap_or_default()
    }

    fn get_previous_principals(&self) -> Option<Arc<dyn PrincipalCollection>> {
        let mut previous_principals = None;
        let stack = self.get_run_as_principals_stack();

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

    fn release_run_as(&mut self) -> Option<&dyn PrincipalCollection> {
        self.pop_identity()
    }
}

impl<T> DelegatingSubject<T>
where
    T: SecurityManager + Clone,
    T: 'static,
{
    fn create_session_context(&self) -> impl SessionContext + 'static {
        let mut session_context = DefaultSessionContext::default();
        if let Some(host) = self.host.as_ref() {
            if host.trim().len() > 0 {
                session_context.set_host(host.as_str());
            }
        }

        session_context
    }

    fn clear_run_as_identities_internal(&mut self) {
        if let Err(se) = self.clear_run_as_identities() {
            debug!(
                "Encountered session exception trying to clear 'runAs' identities during logout.  This can generally safely be ignored. {:?}",
                se
            )
        }
    }

    fn clear_run_as_identities(&mut self) -> Result<Option<SessionValue>, SessionError> {
        let session = self.get_session_or_create(false);
        match session {
            Some(session) => session.remove_attribute(Self::RUN_AS_PRINCIPALS_SESSION_KEY),
            None => Err(SessionError::Invalid),
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
    fn new(target: Box<dyn Session>) -> Self {
        let proxied_session = ProxiedSession::new(target);

        Self { proxied_session }
    }

    pub fn stop(&mut self, subject: &mut DelegatingSubject) {
        self.proxied_session.stop().unwrap();
        subject.session_stopped();
    }
}

impl Session for StoppingAwareProxiedSession {
    fn id(&self) -> &crate::core::session::SessionId {
        self.proxied_session.id()
    }

    fn start_timestamp(&self) -> &DateTime<Utc> {
        self.proxied_session.start_timestamp()
    }

    fn last_access_time(&self) -> &DateTime<Utc> {
        self.proxied_session.last_access_time()
    }

    fn timeout(&self) -> Result<u64, SessionError> {
        self.proxied_session.timeout()
    }

    fn set_timeout(&mut self, max_idle_time_in_millis: u64) -> Result<(), SessionError> {
        self.proxied_session.set_timeout(max_idle_time_in_millis)
    }

    fn host(&self) -> Option<&str> {
        self.proxied_session.host()
    }

    fn touch(&self) -> Result<(), SessionError> {
        self.proxied_session.touch()
    }

    fn stop(&self) -> Result<(), SessionError> {
        self.proxied_session.stop()
    }

    fn attribute_keys(&self) -> Result<std::collections::HashSet<String>, SessionError> {
        self.proxied_session.attribute_keys()
    }

    fn get_attribute(&self, key: &str) -> Option<SessionValue> {
        self.proxied_session.get_attribute(key)
    }

    fn set_attribute(&mut self, key: &str, value: SessionValue) -> Result<(), SessionError> {
        self.proxied_session.set_attribute(key, value)
    }

    fn remove_attribute(&mut self, key: &str) -> Result<Option<SessionValue>, SessionError> {
        self.proxied_session.remove_attribute(key)
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

    fn create_session_context(&self) -> impl SessionContext + 'static;
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
