use std::{any::Any, fmt::Display, sync::Arc};

use next_web_core::async_trait;

use crate::{
    core::{
        authc::{
            authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken,
            bearer_token::BearerToken, host_authentication_token::HostAuthenticationToken,
        },
        session::{Session, SessionId},
        subject::{
            principal_collection::PrincipalCollection, subject_context::SubjectContext, Subject,
        },
        util::{
            map_context::MapContext,
            object::{AnyObject, Object},
        },
    },
    web::mgt::web_security_manager::WebSecurityManager,
};

#[derive(Clone)]
pub struct DefaultSubjectContext {
    map_context: MapContext,
    security_manager: Option<Arc<dyn WebSecurityManager>>,
}

impl DefaultSubjectContext {
    pub const SESSION_CREATION_ENABLED: &str = stringify!(format!(
        "{}.SESSION_CREATION_ENABLED",
        std::any::type_name::<Self>()
    ));

    pub const PRINCIPALS_SESSION_KEY: &str = stringify!(format!(
        "{}_PRINCIPALS_SESSION_KEY",
        std::any::type_name::<Self>()
    ));

    pub const AUTHENTICATED_SESSION_KEY: &str = stringify!(format!(
        "{}_AUTHENTICATED_SESSION_KEY",
        std::any::type_name::<Self>()
    ));

    const SECURITY_MANAGER: &str = stringify!(format!(
        "{}.SECURITY_MANAGER",
        std::any::type_name::<Self>()
    ));

    const SESSION_ID: &str = stringify!(format!("{}.SESSION_ID", std::any::type_name::<Self>()));

    const AUTHENTICATION_TOKEN: &str = stringify!(format!(
        "{}.AUTHENTICATION_TOKEN",
        std::any::type_name::<Self>()
    ));

    const AUTHENTICATION_INFO: &str = stringify!(format!(
        "{}.AUTHENTICATION_INFO",
        std::any::type_name::<Self>()
    ));

    const SUBJECT: &str = stringify!(format!("{}.SUBJECT", std::any::type_name::<Self>()));

    const PRINCIPALS: &str = stringify!(format!("{}.PRINCIPALS", std::any::type_name::<Self>()));

    const SESSION: &str = stringify!(format!("{}.SESSION", std::any::type_name::<Self>()));

    const AUTHENTICATED: &str =
        stringify!(format!("{}.AUTHENTICATED", std::any::type_name::<Self>()));

    const HOST: &str = stringify!(format!("{}.HOST", std::any::type_name::<Self>()));

    pub fn new(context: &dyn SubjectContext) -> Self {
        Self {
            map_context: MapContext::from_context(context),
            security_manager: None,
        }
    }

    pub fn from_security_manager(manager: Arc<dyn WebSecurityManager>) -> Self {
        Self {
            map_context: Default::default(),
            security_manager: Some(manager),
        }
    }

    fn get_type_value<T: AnyObject>(&self, key: &str) -> Option<&T> {
        self.map_context
            .get(key)
            .map(|value| value.as_object::<T>())?
    }

    fn set_value(&mut self, key: impl ToString, value: Object) {
        self.map_context.insert(key.to_string(), value);
    }

    pub fn set_security_manager(&mut self, manager: Arc<dyn WebSecurityManager>) {
        self.security_manager = Some(manager);
    }
}

#[async_trait]
impl SubjectContext for DefaultSubjectContext {
    fn get_session_id(&self) -> &SessionId {
        self.get_type_value::<SessionId>(Self::SESSION_ID).unwrap()
    }

    fn set_session_id(&mut self, session_id: SessionId) {
        self.set_value(Self::SESSION_ID, Object::Obj(Box::new(session_id)));
    }

    fn get_subject(&self) -> Option<&dyn Subject> {
        self.get_type_value::<Box<dyn Subject>>(Self::SUBJECT)
            .map(AsRef::as_ref)
    }

    fn set_subject(&mut self, subject: Box<dyn Subject>) {
        self.set_value(Self::SUBJECT, Object::Obj(Box::new(subject)));
    }

    fn get_principals(&self) -> Option<&Arc<dyn PrincipalCollection>> {
        self.get_type_value::<Arc<dyn PrincipalCollection>>(Self::PRINCIPALS)
    }

    fn set_principals(&mut self, principals: Arc<dyn PrincipalCollection>) {
        if !principals.is_empty() {
            self.set_value(Self::PRINCIPALS, Object::Obj(Box::new(principals)));
        }
    }

    async fn resolve_security_manager(&self) -> Option<&Arc<dyn WebSecurityManager>> {
        self.security_manager.as_ref()
    }

    async fn resolve_principals(&self) -> Option<&Arc<dyn PrincipalCollection>> {
        let mut principals = self.get_principals();
        if principals.is_none() {
            // check to see if they were just authenticated:
            if let Some(info) = self.get_authentication_info() {
                principals = info.get_principals();
            }
        }

        if principals.is_none() {
            if let Some(subject) = self.get_subject() {
                principals = subject.get_principals().await;
            }
        }

        if principals.is_none() {
            if let Some(session) = self.resolve_session() {
                // principals = session
                //     .get_attribute(Self::PRINCIPALS_SESSION_KEY)
                //     .await
                //     .map(|value| value.as_object::<Arc<dyn PrincipalCollection>>())
                //     .unwrap_or_default();
                todo!()
            }
        }

        principals
    }

    fn get_session(&self) -> Option<&Arc<dyn Session>> {
        self.get_type_value::<Arc<dyn Session>>(Self::SESSION)
    }

    fn set_session(&mut self, session: Arc<dyn Session>) {
        self.set_value(Self::SESSION, Object::Obj(Box::new(session)));
    }

    fn resolve_session(&self) -> Option<&Arc<dyn Session>> {
        match self.get_session() {
            Some(session) => Some(session),
            None => {
                let subject = self.get_subject();
                if let Some(subject) = subject {
                    return subject.get_session();
                }
                None
            }
        }
    }

    fn is_authenticated(&self) -> bool {
        self.map_context
            .get(Self::AUTHENTICATED)
            .map(|value| value.as_bool().unwrap_or_default())
            .unwrap_or_default()
    }

    fn set_authenticated(&mut self, authc: bool) {
        self.set_value(Self::AUTHENTICATED, Object::Bool(authc));
    }

    fn is_session_creation_enabled(&self) -> bool {
        self.map_context
            .get(Self::SESSION_CREATION_ENABLED)
            .map(|value| value.as_bool().unwrap_or(true))
            .unwrap_or(true)
    }

    fn set_session_creation_enabled(&mut self, enabled: bool) {
        self.set_value(Self::SESSION_CREATION_ENABLED, Object::Bool(enabled));
    }

    async fn resolve_authenticated(&self) -> bool {
        let mut authc = self
            .map_context
            .get(Self::AUTHENTICATED)
            .map(|value| value.as_bool())
            .unwrap_or_default();

        if authc.is_none() {
            //see if there is an AuthenticationInfo object.  If so, the very presence of one indicates a successful
            //authentication attempt:
            authc.replace(self.get_authentication_info().is_some());
        }

        if let Some(authc) = authc {
            if !authc {
                if let Some(session) = self.resolve_session() {
                    return session
                        .get_attribute(Self::AUTHENTICATED_SESSION_KEY)
                        .await
                        .map(|value| value.as_boolean().unwrap_or_default())
                        .unwrap_or_default();
                }
            }
        }

        false
    }

    fn get_authentication_info(&self) -> Option<&dyn AuthenticationInfo> {
        self.get_type_value::<Arc<dyn AuthenticationInfo>>(Self::AUTHENTICATION_INFO)
            .map(AsRef::as_ref)
    }

    fn set_authentication_info(&mut self, info: Box<dyn AuthenticationInfo>) {
        self.map_context.insert(
            Self::AUTHENTICATION_INFO.to_string(),
            Object::Obj(Box::new(info)),
        );
    }

    fn get_authentication_token(&self) -> Option<&dyn AuthenticationToken> {
        self.get_type_value::<Arc<dyn AuthenticationToken>>(Self::AUTHENTICATION_TOKEN)
            .map(AsRef::as_ref)
    }

    fn set_authentication_token(&mut self, token: Box<dyn AuthenticationToken>) {
        self.set_value(Self::AUTHENTICATION_TOKEN, Object::Obj(Box::new(token)));
    }

    fn get_host(&self) -> Option<&str> {
        self.map_context.get(Self::HOST).map(|obj| obj.as_str())?
    }

    fn set_host(&mut self, host: String) {
        if !host.is_empty() {
            self.map_context
                .insert(Self::HOST.to_string(), Object::Str(host));
        }
    }

    async fn resolve_host(&self) -> Option<String> {
        let mut host = self.get_host();
        if let None = host {
            //check to see if there is an AuthenticationToken from which to retrieve it:
            let token = self.get_authentication_token();
            if let Some(token) = token {
                if let Some(host_authentication_token) =
                    (token as &dyn Any).downcast_ref::<BearerToken>()
                {
                    host = host_authentication_token.get_host();
                }
            }
        }

        if let None = host {
            if let Some(session) = self.resolve_session() {
                host = session.host();
            }
        }

        host.map(ToString::to_string)
    }

    fn is_empty(&self) -> bool {
        self.map_context.is_empty()
    }

    fn values(&self) -> Vec<(String, Object)> {
        self.map_context
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }
}

impl Display for DefaultSubjectContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DefaultSubjectContext")
    }
}

impl Default for DefaultSubjectContext {
    fn default() -> Self {
        Self {
            map_context: Default::default(),
            security_manager: Default::default(),
        }
    }
}
