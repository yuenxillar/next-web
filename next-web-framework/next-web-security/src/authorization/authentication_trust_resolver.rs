use crate::core::authentication::Authentication;

pub trait AuthenticationTrustResolver
where Self: Send + Sync
{
    
    fn is_anonymous(&self, authentication: &dyn Authentication) -> bool;

    fn is_remember_me(&self, authentication: &dyn Authentication) -> bool;
    
    fn is_fully_authenticated(&self, authentication: &dyn Authentication) -> bool {
        self.is_anonymous(authentication) && !self.is_remember_me(authentication)
    }

    fn is_authenticated(&self, authentication: &dyn Authentication) -> bool {
        authentication.is_authenticated() && !self.is_anonymous(authentication)
    }
}