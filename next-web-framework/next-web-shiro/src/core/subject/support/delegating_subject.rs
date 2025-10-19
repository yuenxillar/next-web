use next_web_core::traits::required::Required;

use crate::core::{
    authc::{authentication_error::AuthenticationError, authentication_token::AuthenticationToken}, authz::authorization_error::AuthorizationError, mgt::security_manager::SecurityManager, object::Object, session::{mgt::{default_session_context::DefaultSessionContext, session_context::SessionContext}, proxied_session::ProxiedSession, Session}, subject::{principal_collection::PrincipalCollection, Subject}
};
use std::{any::Any, sync::Arc};

#[derive(Clone)]
pub struct DelegatingSubject {
    principals: Option<Arc<dyn PrincipalCollection>>,
    authenticated: bool,
    host: Option<String>,
    session: Option<Arc<dyn Session>>,
    session_creation_enabled: bool,
    security_manager: Arc<dyn SecurityManager>,
}

impl DelegatingSubject {
    const RUN_AS_PRINCIPALS_SESSION_KEY: &str = stringify!(format!(
        "{}{}",
        std::any::type_name::<Self>(),
        ".RUN_AS_PRINCIPALS_SESSION_KEY"
    ));

    pub fn new<T>(
        principals: Option<Arc<dyn PrincipalCollection>>,
        authenticated: bool,
        host: Option<String>,
        session: Option<Arc<dyn Session>>,
        session_creation_enabled: bool,
        security_manager: T,
    ) -> Self
    where
        T: SecurityManager + 'static,
    {
        let mut subject = Self {
            principals,
            authenticated,
            host,
            session_creation_enabled,
            session: None,
            security_manager: Arc::new(security_manager) as Arc<dyn SecurityManager>,
        };
        if session.is_some() {
           
        }
        let session = match session {
            Some(session) =>  subject.decorate(session),
            None => return subject,
        };

        subject.session = Some(session);
        subject
    }

    pub fn decorate(&self, session: Arc<dyn Session>) -> Arc<dyn Session> {
        Arc::new(
            StoppingAwareProxiedSession::new(session, self.clone())
        )
    }

    pub fn get_security_manager(&self) -> &dyn SecurityManager {
        self.security_manager.as_ref()
    }

    pub fn has_principals(&self) -> bool {
        match self.principals.as_ref() {
            Some(principals) => !principals.is_empty(),
            None => false
        }
    }

    pub fn get_host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    // pub fn get_session(&self) -> Option<&dyn Session> {
    //     self.session.as_deref()
    // }

    pub fn get_primary_principal<'a>(&'a self,
        principals: Option<& dyn PrincipalCollection>
    ) -> Option<&'a  Object> 
    {
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

    fn get_run_as_principals_stack(&self) -> Vec<& dyn PrincipalCollection> {
        // let session = self.get_session()

        vec![]
    }

    pub fn assert_authz_check_possible(
        &self
    ) -> Result<(), AuthorizationError>
    {
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
}

impl<T> From<T> for DelegatingSubject
where
    T: SecurityManager + 'static,
{
    fn from(security_manager: T) -> Self {
        Self::new(None, false, None, None, true, security_manager)
    }
}

impl Subject for DelegatingSubject {
    fn get_principal(&self) -> Option<& Object> {
        self.get_primary_principal(self.get_principals().as_deref())
    }

    fn get_principals(&self) -> Option<Arc<dyn PrincipalCollection>>
    {
        let run_as_principals = self.get_run_as_principals_stack();
        if run_as_principals.is_empty() {
            self.principals.as_ref().map(|x| x.clone())
        }else {
            run_as_principals.get(0).map(|x| *x)
        }
    }

    fn is_permitted(&self, permission: &str) -> bool {
        self.has_principals() && self.security_manager.is_permitted(self.get_principals(), permission)
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
        self.has_principals() && self.security_manager.is_permitted_all(self.get_principals(), permissions)
    }

    fn check_permission(
        &self,
        permission: &str,
    ) -> Result<(), AuthorizationError> {
        self.assert_authz_check_possible()?;
        self.security_manager.check_permission(self.get_principals(), permission)
    }

    fn check_permissions(
        &self,
        permissions: &[&str],
    ) -> Result<(), AuthorizationError> {
        self.assert_authz_check_possible()?;
        self.security_manager.check_permissions(self.get_principals(), permissions)
    }

    fn has_role(&self, role_identifier: &str) -> bool {
        self.has_principals() && self.security_manager.has_role(self.get_principals(), role_identifier)
    }

    fn has_all_roles(&self, roles: &[&str]) -> bool {
        self.has_principals() && self.security_manager.has_all_roles(self.get_principals(), roles)
    }

    fn check_role(
        &self,
        role: &str,
    ) -> Result<(), AuthorizationError> {
        self.assert_authz_check_possible()?;
        self.security_manager.check_role(self.get_principals(), role)
    }

    fn check_roles(
        &self,
        roles: &[&str],
    ) -> Result<(), AuthorizationError> {
        self.assert_authz_check_possible()?;
        self.security_manager.check_roles(self.get_principals(), roles)
    }

    fn get_session(&self) -> Option<&dyn Session> {
        self.get_session_or_create(true)
    }

    fn get_session_or_create(&self, create: bool) -> Option<&dyn Session> {
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
            let session = self.security_manager.start(& session_context);
            self.session = Some(self.decorate(session));
       }

       self.session.as_deref()
    }

    fn login(
        &mut self,
        token: &dyn AuthenticationToken,
    ) -> Result<(), AuthenticationError> {
        
        let subject = self.security_manager.login(self, token)?;

        let mut principals = None;
        let mut host = None;

        let any: &dyn Any = &subject;
        if let Some(delegating) = any.downcast_ref::<Self>() {
            principals = delegating.principals.as_deref();
            host = delegating.host.as_ref();
        }else {
            principals = subject.get_principals();
        }

        if match principals {
            Some(val) => val.is_empty(),
            None => true
        } {
            return Err(AuthenticationError::IllegalState("Principals returned from securityManager.login( token ) returned a null or 
            empty value.  This value must be non null and populated with one or more elements.".to_string()));
        }

        self.principals = principals.map(|x| Arc::new(x.clone()));
        self.authenticated = true;

        if let Some(token) =(token as &dyn Any).downcast_ref::<HostAuthenticationToken>() {
            host = token.get_host();
        }

        self.host = host.map(|x| x.clone());
        
        let session = subject.get_session_or_create(false);
        
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

    fn logout(&mut self) {
        todo!()
    }

    fn run_as(&mut self, principals: Vec<Object>) -> Result<(), String> {
        todo!()
    }

    fn is_run_as(&self) -> bool {
        todo!()
    }

    fn get_previous_principals(&self) -> Option<Vec<Object>> {
        todo!()
    }

    fn release_run_as(&mut self) -> Option<Vec<Object>> {
        todo!()
    }
}


impl DelegatingSubject {

    fn create_session_context(&self) -> impl SessionContext + 'static {
        let mut session_context = DefaultSessionContext::default();
        if let Some(host) = self.host.as_ref() {
            if host.trim().len() > 0 {
                session_context.set_host(host.as_str());
            }
        }

        session_context
    }
}
pub struct StoppingAwareProxiedSession
where
    Self: Required<ProxiedSession>, {}

impl StoppingAwareProxiedSession {
    fn new(session: Arc<dyn Session>, subject: DelegatingSubject) -> Self {
        //
        Self {}
    }
}

impl Required<ProxiedSession> for StoppingAwareProxiedSession {
    fn get_object(&self) -> &ProxiedSession {
        todo!()
    }

    fn get_object_mut(&mut self) -> &mut ProxiedSession {
        todo!()
    }
}

impl Session for StoppingAwareProxiedSession {
    fn id(&self) -> SessionId {
        todo!()
    }

    fn start_timestamp(&self) -> std::time::SystemTime {
        todo!()
    }

    fn last_access_time(&self) -> std::time::SystemTime {
        todo!()
    }

    fn timeout(&self) -> Result<u64, crate::core::session::InvalidSessionError> {
        todo!()
    }

    fn set_timeout(&mut self, max_idle_time_in_millis: u64) -> Result<(), crate::core::session::InvalidSessionError> {
        todo!()
    }

    fn host(&self) -> Option<&str> {
        todo!()
    }

    fn touch(&mut self) -> Result<(), crate::core::session::InvalidSessionError> {
        todo!()
    }

    fn stop(&mut self) -> Result<(), crate::core::session::InvalidSessionError> {
        todo!()
    }

    fn attribute_keys(&self) -> Result<std::collections::HashSet<String>, crate::core::session::InvalidSessionError> {
        todo!()
    }

    fn get_attribute(&self, key: &str) -> Result<Option<SessionValue>, crate::core::session::InvalidSessionError> {
        todo!()
    }

    fn set_attribute(
        &mut self,
        key: &str,
        value: Option<SessionValue>,
    ) -> Result<(), crate::core::session::InvalidSessionError> {
        todo!()
    }

    fn remove_attribute(&mut self, key: &str) -> Result<Option<SessionValue>, crate::core::session::InvalidSessionError> {
        todo!()
    }
}
