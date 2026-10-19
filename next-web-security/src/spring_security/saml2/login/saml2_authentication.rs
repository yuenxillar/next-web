use std::{any::TypeId, borrow::Cow, sync::Arc};

use next_web_core::error::BoxError;

use crate::{
    core::{
        Authentication, AuthenticationBuilder, GrantedAuthority, Principal, SimpleAuthentication,
    },
    web::authentication::AuthPrincipal,
};

/// The authentication a believed response results in.
///
/// The principal is the name id the asserting party asserted, the authorities
/// are the ones of the user that name id maps to, and the response is kept so a
/// caller can log or inspect what the user was authenticated from.
#[derive(Clone, Debug)]
pub struct Saml2Authentication {
    base: SimpleAuthentication,
    response_xml: String,
}

impl Saml2Authentication {
    /// Creates an authentication for the given name id.
    pub fn new(
        name_id: impl Into<String>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
        response_xml: impl Into<String>,
    ) -> Self {
        let mut builder = SimpleAuthentication::builder();
        AuthenticationBuilder::principal(
            &mut builder,
            Some(Arc::new(name_id.into()) as AuthPrincipal),
        );
        AuthenticationBuilder::authorities(
            &mut builder,
            Box::new(move |values| values.extend(authorities)),
        );
        AuthenticationBuilder::authenticated(&mut builder, true);

        Self {
            base: SimpleAuthentication::from_builder(builder),
            response_xml: response_xml.into(),
        }
    }

    /// Returns the response the authentication came from.
    pub fn response_xml(&self) -> &str {
        &self.response_xml
    }
}

impl Authentication for Saml2Authentication {
    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        self.base.authorities()
    }

    fn credentials(&self) -> Option<&AuthPrincipal> {
        self.base.credentials()
    }

    fn details(&self) -> Option<&AuthPrincipal> {
        self.base.details()
    }

    fn principal(&self) -> Option<&AuthPrincipal> {
        self.base.principal()
    }

    fn is_authenticated(&self) -> bool {
        self.base.is_authenticated()
    }

    fn set_authenticated(&mut self, is_authenticated: bool) -> Result<(), BoxError> {
        self.base.set_authenticated(is_authenticated)
    }

    fn to_builder(&self) -> Box<dyn AuthenticationBuilder> {
        Box::new(Saml2AuthenticationBuilder {
            name_id: self.name().into_owned(),
            authorities: self.base.authorities().to_vec(),
            response_xml: self.response_xml.clone(),
            authenticated: self.base.is_authenticated(),
        })
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

/// Rebuilds a [`Saml2Authentication`].
///
/// The authentication manager rebuilds the result it hands back — it erases the
/// credentials of a successful authentication — so the rebuild has to yield the
/// SAML 2.0 authentication again. A caller that downcasts the authentication to
/// read the response would otherwise find a plain one.
#[derive(Clone)]
struct Saml2AuthenticationBuilder {
    name_id: String,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    response_xml: String,
    authenticated: bool,
}

impl AuthenticationBuilder for Saml2AuthenticationBuilder {
    fn authorities(&mut self, authorities: Box<dyn FnOnce(&mut Vec<Arc<dyn GrantedAuthority>>)>) {
        authorities(&mut self.authorities);
    }

    fn credentials(&mut self, _credentials: Option<AuthPrincipal>) {}

    fn details(&mut self, _details: Option<AuthPrincipal>) {}

    fn principal(&mut self, principal: Option<AuthPrincipal>) {
        if let Some(principal) = principal {
            self.name_id = principal.to_string();
        }
    }

    fn authenticated(&mut self, authenticated: bool) {
        self.authenticated = authenticated;
    }

    fn build(&mut self) -> Arc<dyn Authentication> {
        let mut authentication = Saml2Authentication::new(
            self.name_id.clone(),
            self.authorities.clone(),
            self.response_xml.clone(),
        );
        let _ = authentication.set_authenticated(self.authenticated);

        Arc::new(authentication)
    }
}

impl Principal for Saml2Authentication {
    fn name(&self) -> Cow<'_, str> {
        self.base
            .principal()
            .map(|principal| Cow::Owned(principal.to_string()))
            .unwrap_or(Cow::Borrowed(""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::authority::SimpleGrantedAuthority;

    #[test]
    fn the_authentication_is_authenticated_under_the_name_id() {
        let authentication =
            Saml2Authentication::new("max@example.org", Vec::new(), "<saml2p:Response/>");

        assert!(authentication.is_authenticated());
        assert_eq!(authentication.name(), "max@example.org");
    }

    #[test]
    fn the_authentication_carries_the_authorities_of_the_user() {
        let authority: Arc<dyn GrantedAuthority> =
            Arc::new(SimpleGrantedAuthority::new("ROLE_USER"));

        let authentication =
            Saml2Authentication::new("max@example.org", vec![authority], "<saml2p:Response/>");

        assert_eq!(authentication.authorities().len(), 1);
        assert_eq!(
            authentication.authorities()[0].authority().as_deref(),
            Some("ROLE_USER")
        );
    }

    #[test]
    fn the_authentication_keeps_the_response_it_came_from() {
        let authentication =
            Saml2Authentication::new("max@example.org", Vec::new(), "<saml2p:Response/>");

        assert_eq!(authentication.response_xml(), "<saml2p:Response/>");
    }
}
