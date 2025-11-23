use crate::core::authc::authentication_token::AuthenticationToken;

pub trait RememberMeAuthenticationToken
where
    Self: AuthenticationToken,
{
    fn is_remember_me(&self) -> bool;
}
