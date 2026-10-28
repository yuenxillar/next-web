use std::{
    any::TypeId,
    borrow::Cow,
    fmt::{Debug, Display},
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::error::BoxError;

use crate::{
    authentication::{BaseAuthenticationBuilder, BaseAuthenticationToken},
    core::{Authentication, AuthenticationBuilder, GrantedAuthority, Principal},
    web::authentication::AuthPrincipal,
};

/// The authenticated `Authentication` produced after a one-time token has been
/// successfully consumed. It carries the resolved principal (username) together
/// with the authorities, including `FactorGrantedAuthority.OTT_AUTHORITY`.
#[derive(Clone)]
pub struct OneTimeTokenAuthentication {
    principal: AuthPrincipal,
    base: BaseAuthenticationToken,
}

impl OneTimeTokenAuthentication {
    pub fn new(principal: impl Into<String>, authorities: Vec<Arc<dyn GrantedAuthority>>) -> Self {
        let mut base = BaseAuthenticationToken::new(Some(authorities));
        base.set_authenticated(true)
            .inspect_err(|err| {
                tracing::error!("BaseAuthenticationToken set_authenticated failed: {}", err)
            })
            .ok();

        Self {
            principal: Arc::new(principal.into()),
            base,
        }
    }

    pub fn set_details_value(&mut self, details: Option<AuthPrincipal>) {
        self.base.set_details(details);
    }

    pub fn from_builder(builder: &mut OneTimeTokenAuthenticationBuilder) -> Self {
        Self {
            principal: builder
                .principal
                .take()
                .unwrap_or_else(|| Arc::new(String::new())),
            base: BaseAuthenticationToken::from_builder(builder),
        }
    }
}

impl Authentication for OneTimeTokenAuthentication {
    fn credentials(&self) -> Option<&AuthPrincipal> {
        None
    }

    fn details(&self) -> Option<&AuthPrincipal> {
        self.base.details()
    }

    fn principal(&self) -> Option<&AuthPrincipal> {
        Some(&self.principal)
    }

    fn is_authenticated(&self) -> bool {
        self.base.is_authenticated()
    }

    fn set_authenticated(&mut self, is_authenticated: bool) -> Result<(), BoxError> {
        if is_authenticated {
            return Err(
                "Cannot set this token to trusted - use the new constructor instead".into(),
            );
        }
        self.base.set_authenticated(false)?;
        Ok(())
    }

    fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        self.base.authorities()
    }

    fn to_builder(&self) -> Box<dyn AuthenticationBuilder> {
        Box::new(OneTimeTokenAuthenticationBuilder::with_token(self))
    }

    fn of(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

impl Principal for OneTimeTokenAuthentication {
    fn name(&self) -> Cow<'_, str> {
        Cow::Owned(self.principal.to_string())
    }
}

impl Display for OneTimeTokenAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} [Principal={:?}, Authenticated={}, Authorities={:?}]",
            std::any::type_name::<Self>(),
            self.principal().map(|principal| principal.to_string()),
            self.is_authenticated(),
            self.base
                .authorities()
                .iter()
                .map(|a| a.authority())
                .collect::<Vec<_>>()
        )
    }
}

impl Debug for OneTimeTokenAuthentication {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

/// A builder of `OneTimeTokenAuthentication` instances.
pub struct OneTimeTokenAuthenticationBuilder {
    principal: Option<AuthPrincipal>,
    base: BaseAuthenticationBuilder,
}

impl OneTimeTokenAuthenticationBuilder {
    fn with_token(token: &OneTimeTokenAuthentication) -> Self {
        Self {
            principal: Some(token.principal.clone()),
            base: BaseAuthenticationBuilder::with_token(token),
        }
    }
}

impl AuthenticationBuilder for OneTimeTokenAuthenticationBuilder {
    fn authorities(&mut self, authorities: Box<dyn FnOnce(&mut Vec<Arc<dyn GrantedAuthority>>)>) {
        self.base.authorities(authorities);
    }

    fn details(&mut self, details: Option<AuthPrincipal>) {
        self.base.details(details);
    }

    fn principal(&mut self, principal: Option<AuthPrincipal>) {
        self.principal = principal;
    }

    fn credentials(&mut self, _credentials: Option<AuthPrincipal>) {}

    fn authenticated(&mut self, authenticated: bool) {
        self.base.authenticated(authenticated);
    }

    fn build(&mut self) -> Arc<dyn Authentication> {
        Arc::new(OneTimeTokenAuthentication::from_builder(self))
    }
}

impl Deref for OneTimeTokenAuthentication {
    type Target = BaseAuthenticationToken;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for OneTimeTokenAuthentication {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Deref for OneTimeTokenAuthenticationBuilder {
    type Target = BaseAuthenticationBuilder;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for OneTimeTokenAuthenticationBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}
