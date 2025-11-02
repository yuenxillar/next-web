use crate::core::mgt::remember_me_manager::RememberMeManager;


#[derive(Clone)]
pub struct CookieRememberMeManager {

}

impl  CookieRememberMeManager {
    
    pub fn new(
        key: Vec<u8>
    ) -> Self {
        Self {  }
    }
}

impl RememberMeManager for  CookieRememberMeManager {
    fn get_remembered_principals(
        &self,
        subject_context: &dyn crate::core::subject::subject_context::SubjectContext,
    ) -> Option<std::sync::Arc<dyn crate::core::subject::principal_collection::PrincipalCollection>> {
        todo!()
    }

    fn forget_identity(&self, subject_context: &dyn crate::core::subject::subject_context::SubjectContext) {
        todo!()
    }

    fn on_successful_login(
        &self,
        subject: &dyn crate::core::subject::Subject,
        token: &dyn crate::core::authc::authentication_token::AuthenticationToken,
        info: &dyn crate::core::authc::authentication_info::AuthenticationInfo,
    ) {
        todo!()
    }

    fn on_failed_login(
        &self,
        subject: &dyn crate::core::subject::Subject,
        token: &dyn crate::core::authc::authentication_token::AuthenticationToken,
        ae: &crate::core::authc::authentication_error::AuthenticationError,
    ) {
        todo!()
    }

    fn on_logout(&self, subject: &dyn crate::core::subject::Subject) {
        todo!()
    }
}


impl Default for CookieRememberMeManager  {
    fn default() -> Self {
        Self {  }
    }
}