use std::{
    any::{Any, TypeId},
    sync::Arc,
};

use next_web_core::async_trait;

use crate::{
    authentication::AuthenticationProvider,
    core::{
        authority::mapping::{GrantedAuthoritiesMapper, NullAuthoritiesMapper},
        Authentication, AuthenticationError,
    },
    ldap::{
        authentication::{
            ldap_authentication_request::LdapAuthenticationRequest,
            ldap_authenticator::LdapAuthenticator,
        },
        userdetails::ldap_authorities_populator::{
            LdapAuthoritiesPopulator, NullLdapAuthoritiesPopulator,
        },
    },
};

pub struct LdapAuthenticationProvider {
    authenticator: Arc<dyn LdapAuthenticator>,
    authorities_populator: Arc<dyn LdapAuthoritiesPopulator>,
    authorities_mapper: Arc<dyn GrantedAuthoritiesMapper>,
    hide_user_not_found_exceptions: bool,
}

impl LdapAuthenticationProvider {
    pub fn new(authenticator: Arc<dyn LdapAuthenticator>) -> Self {
        Self {
            authenticator,
            authorities_populator: Arc::new(NullLdapAuthoritiesPopulator),
            authorities_mapper: Arc::new(NullAuthoritiesMapper),
            hide_user_not_found_exceptions: true,
        }
    }

    pub fn with_authorities_populator(
        authenticator: Arc<dyn LdapAuthenticator>,
        authorities_populator: Arc<dyn LdapAuthoritiesPopulator>,
    ) -> Self {
        Self {
            authenticator,
            authorities_populator,
            authorities_mapper: Arc::new(NullAuthoritiesMapper),
            hide_user_not_found_exceptions: true,
        }
    }

    pub fn set_hide_user_not_found_exceptions(&mut self, hide: bool) {
        self.hide_user_not_found_exceptions = hide;
    }

    pub fn set_authorities_mapper(
        &mut self,
        authorities_mapper: Arc<dyn GrantedAuthoritiesMapper>,
    ) {
        self.authorities_mapper = authorities_mapper;
    }

    pub async fn authenticate_request(
        &self,
        authentication: &LdapAuthenticationRequest,
    ) -> Result<Vec<String>, AuthenticationError> {
        if authentication.username().trim().is_empty() {
            return Err(AuthenticationError::new("Empty Username"));
        }
        if authentication.password().is_empty() {
            return Err(AuthenticationError::new("Empty Password"));
        }

        let user_data = self.authenticator.authenticate(authentication).await?;
        let authorities = self
            .authorities_populator
            .get_granted_authorities(&user_data, authentication.username())
            .await;
        let authorities = self.authorities_mapper.map_authorities(
            &authorities
                .into_iter()
                .map(|authority| authority as Arc<dyn crate::core::GrantedAuthority>)
                .collect::<Vec<_>>(),
        );

        let mut mapped = Vec::new();
        for authority in authorities {
            if let Some(authority) = authority.authority() {
                mapped.push(authority.to_string());
            }
        }

        Ok(mapped)
    }
}

#[async_trait]
impl AuthenticationProvider for LdapAuthenticationProvider {
    async fn authenticate(
        &self,
        authentication: &Arc<dyn Authentication>,
    ) -> Result<Option<Arc<dyn Authentication>>, AuthenticationError> {
        let Some(mut authentication) = (authentication.as_ref() as &dyn Any)
            .downcast_ref::<LdapAuthenticationRequest>()
            .map(Clone::clone)
        else {
            return Err(AuthenticationError::new(
                "Only LdapAuthenticationRequest is supported",
            ));
        };

        let authorities = self.authenticate_request(&authentication).await?;
        authentication.set_authenticated(true);
        authentication.set_authorities(authorities);

        Ok(Some(Arc::new(authentication)))
    }

    fn supports(&self, authentication: TypeId) -> bool {
        authentication == TypeId::of::<LdapAuthenticationRequest>()
    }
}
