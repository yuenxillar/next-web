use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    authentication::authentication_provider::AuthenticationProvider,
    core::{
        authentication_error::AuthenticationError,
        authority_mapping::{GrantedAuthoritiesMapper, NullAuthoritiesMapper},
        Authentication,
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
            authorities
                .into_iter()
                .map(|authority| {
                    authority as Arc<dyn crate::core::granted_authority::GrantedAuthority>
                })
                .collect(),
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
        authentication: &dyn Authentication,
    ) -> Result<Arc<dyn Authentication>, AuthenticationError> {
        let Some(authentication) = authentication
            .as_any()
            .downcast_ref::<LdapAuthenticationRequest>()
        else {
            return Err(AuthenticationError::new(
                "Only LdapAuthenticationRequest is supported",
            ));
        };

        let authorities = self.authenticate_request(authentication).await?;
        let mut authenticated = authentication.clone();
        authenticated.set_authenticated(true);
        authenticated.set_authorities(authorities);
        Ok(Arc::new(authenticated))
    }

    fn supports(&self, authentication: &str) -> bool {
        authentication == std::any::type_name::<LdapAuthenticationRequest>()
    }
}
