use crate::core::authc::authentication_token::AuthenticationToken;

pub trait HostAuthenticationToken
where
    Self: Send + Sync,
    Self: AuthenticationToken,
{
    fn get_host(&self) -> Option<&str>;
}
