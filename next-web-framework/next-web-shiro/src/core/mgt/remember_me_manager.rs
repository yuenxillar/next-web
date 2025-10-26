use std::sync::Arc;

use crate::core::{
    authc::{
        authentication_error::AuthenticationError, authentication_info::AuthenticationInfo,
        authentication_token::AuthenticationToken,
    },
    subject::{
        Subject, principal_collection::PrincipalCollection, subject_context::SubjectContext,
    },
};

pub trait RememberMeManager
where
    Self: Send + Sync,
{
    fn get_remembered_principals(
        &self,
        subject_context: &dyn SubjectContext,
    ) -> Option<Arc<dyn PrincipalCollection>>;

    fn forget_identity(&self, subject_context: &dyn SubjectContext);

    fn on_successful_login(
        &self,
        subject: &dyn Subject,
        token: &dyn AuthenticationToken,
        info: &dyn AuthenticationInfo,
    );

    fn on_failed_login(
        &self,
        subject: &dyn Subject,
        token: &dyn AuthenticationToken,
        ae: &AuthenticationError,
    );

    fn on_logout(&self, subject: &dyn Subject);
}

impl RememberMeManager for () {
    fn forget_identity(&self, subject_context: &dyn SubjectContext) {
        todo!()
    }

    fn on_successful_login(
        &self,
        subject: &dyn Subject,
        token: &dyn AuthenticationToken,
        info: &dyn AuthenticationInfo,
    ) {
        todo!()
    }

    fn on_failed_login(
        &self,
        subject: &dyn Subject,
        token: &dyn AuthenticationToken,
        ae: &AuthenticationError,
    ) {
        todo!()
    }

    fn on_logout(&self, subject: &dyn Subject) {
        todo!()
    }

    fn get_remembered_principals(
        &self,
        subject_context: &dyn SubjectContext,
    ) -> Option<Arc<dyn PrincipalCollection>> {
        todo!()
    }
}
