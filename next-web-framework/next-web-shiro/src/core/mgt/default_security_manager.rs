use crate::core::{mgt::{
    default_subject_dao::DefaultSubjectDAO, default_subject_factory::DefaultSubjectFactory, remember_me_manager::RememberMeManager, security_manager::SecurityManager, sessions_security_manager::SessionsSecurityManager, subject_dao::SubjectDAO, subject_factory::SubjectFactory
}, realm::Realm};

#[derive(Clone)]
pub struct DefaultSecurityManager<R = (), D = DefaultSubjectDAO, F = DefaultSubjectFactory> {
    remember_me_manager: Option<R>,
    subject_dao: D,
    subject_factory: F,

    sessions_security_manager: SessionsSecurityManager,
}


impl<T> From<T> for DefaultSecurityManager
where 
T: Realm
{
    fn from(single_realm: T) -> Self {
        let manager = Self::default();
        manager.set_realm(single_realm);
        
        manager
    }
}


impl<T> From<Vec<T>> for DefaultSecurityManager
where 
T: Realm
{
    fn from(realms: Vec<T>) -> Self {
        let manager = Self::default();
        manager.set_realms(realms);
        
        manager
    }
}

impl Default for DefaultSecurityManager {
    fn default() -> Self {
        let sessions_security_manager = SessionsSecurityManager::default();
        Self {
            remember_me_manager: Default::default(),
            subject_dao: Default::default(),
            subject_factory: Default::default(),

            sessions_security_manager,
        }
    }
}

impl<R, D, F> SecurityManager for DefaultSecurityManager<R, D, F>
where
    R: RememberMeManager,
    D: SubjectDAO,
    F: SubjectFactory,
{
    fn login(
        &self,
        subject: &dyn crate::core::subject::Subject,
        authentication_token: &dyn crate::core::authc::authentication_token::AuthenticationToken,
    ) -> Result<
        Box<dyn crate::core::subject::Subject>,
        crate::core::authc::authentication_error::AuthenticationError,
    > {
        todo!()
    }

    fn logout(
        &self,
        subject: &dyn crate::core::subject::Subject,
    ) -> Result<(), next_web_core::error::BoxError> {
        todo!()
    }

    fn create_subject(
        &self,
        context: &dyn crate::core::subject::subject_context::SubjectContext,
    ) -> &dyn crate::core::subject::Subject {
        todo!()
    }
}
