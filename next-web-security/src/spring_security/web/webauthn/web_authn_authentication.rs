use std::{any::TypeId, borrow::Cow, sync::Arc};

use next_web_core::error::BoxError;

use crate::{
    core::{
        Authentication, AuthenticationBuilder, GrantedAuthority, Principal, SimpleAuthentication,
    },
    web::authentication::AuthPrincipal,
};

/// The authentication a verified assertion results in.
///
/// The principal is the user the credential was registered for, the authorities
/// are the ones of that user, and the credential id is kept so that a caller can
/// log or inspect which passkey was used.
#[derive(Clone, Debug)]
pub struct WebAuthnAuthentication {
    base: SimpleAuthentication,
    credential_id: Vec<u8>,
}

impl WebAuthnAuthentication {
    /// Creates an authentication for the given user and credential.
    pub fn new(
        user_name: impl Into<String>,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
        credential_id: Vec<u8>,
    ) -> Self {
        let mut builder = SimpleAuthentication::builder();
        AuthenticationBuilder::principal(
            &mut builder,
            Some(Arc::new(user_name.into()) as AuthPrincipal),
        );
        AuthenticationBuilder::authorities(
            &mut builder,
            Box::new(move |values| values.extend(authorities)),
        );
        AuthenticationBuilder::authenticated(&mut builder, true);

        Self {
            base: SimpleAuthentication::from_builder(builder),
            credential_id,
        }
    }

    /// Returns the identifier of the credential the assertion was made with.
    pub fn credential_id(&self) -> &[u8] {
        &self.credential_id
    }
}

impl Authentication for WebAuthnAuthentication {
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
        Box::new(WebAuthnAuthenticationBuilder {
            user_name: self.name().into_owned(),
            authorities: self.base.authorities().to_vec(),
            credential_id: self.credential_id.clone(),
            authenticated: self.base.is_authenticated(),
        })
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

/// Rebuilds a [`WebAuthnAuthentication`] after the manager erased its credentials.
#[derive(Clone)]
struct WebAuthnAuthenticationBuilder {
    user_name: String,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
    credential_id: Vec<u8>,
    authenticated: bool,
}

impl AuthenticationBuilder for WebAuthnAuthenticationBuilder {
    fn authorities(&mut self, authorities: Box<dyn FnOnce(&mut Vec<Arc<dyn GrantedAuthority>>)>) {
        authorities(&mut self.authorities);
    }

    fn credentials(&mut self, _credentials: Option<AuthPrincipal>) {}

    fn details(&mut self, _details: Option<AuthPrincipal>) {}

    fn principal(&mut self, principal: Option<AuthPrincipal>) {
        if let Some(principal) = principal {
            self.user_name = principal.to_string();
        }
    }

    fn authenticated(&mut self, authenticated: bool) {
        self.authenticated = authenticated;
    }

    fn build(&mut self) -> Arc<dyn Authentication> {
        let mut authentication = WebAuthnAuthentication::new(
            self.user_name.clone(),
            self.authorities.clone(),
            self.credential_id.clone(),
        );
        let _ = authentication.set_authenticated(self.authenticated);

        Arc::new(authentication)
    }
}

impl Principal for WebAuthnAuthentication {
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
    use crate::core::Authentication;

    #[test]
    fn the_authentication_names_the_user_of_the_credential() {
        let authentication = WebAuthnAuthentication::new("Max", Vec::new(), b"the-credential".to_vec());

        assert!(authentication.is_authenticated());
        assert_eq!(authentication.name(), "Max");
        assert_eq!(authentication.credential_id(), b"the-credential");
    }

    #[test]
    fn the_authentication_survives_the_rebuild_of_the_manager() {
        let authentication =
            WebAuthnAuthentication::new("Max", Vec::new(), b"the-credential".to_vec());

        let rebuilt = authentication.to_builder().build();

        assert_eq!(rebuilt.of(), TypeId::of::<WebAuthnAuthentication>());
        assert_eq!(rebuilt.name(), "Max");
    }
}
