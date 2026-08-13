use super::{
    authentication_info::AuthenticationInfo, authentication_token::AuthenticationToken,
    AuthenticationError,
};

pub trait Authenticator
where
    Self: Send + Sync,
{
    fn authenticate(
        &self,
        authentication_token: &dyn AuthenticationToken,
    ) -> Result<Box<dyn AuthenticationInfo>, AuthenticationError>;
}
