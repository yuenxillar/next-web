use std::sync::Arc;

use next_web_core::async_trait;

use crate::{
    authentication::authentication_provider::AuthenticationProvider,
    core::{
        authentication::Authentication,
        authentication_error::AuthenticationError,
        authority_mapping::{GrantedAuthoritiesMapper, NullAuthoritiesMapper},
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
                .map(|authority| authority as Arc<dyn crate::core::granted_authority::GrantedAuthority>)
                .collect(),
        );

        let mut mapped = Vec::new();
        for authority in authorities {
            if let Some(authority) = authority.get_authority().await {
                mapped.push(authority);
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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use next_web_core::async_trait;

    use crate::{
        authentication::authentication_provider::AuthenticationProvider,
        core::userdetails::user_details_service::UserDetailsService,
        ldap::{
            authentication::{
                ldap_authentication_request::LdapAuthenticationRequest,
                ldap_authenticator::LdapAuthenticator,
            },
            dir_context_operations::DirContextOperations,
            search::ldap_user_search::LdapUserSearch,
            userdetails::{
                ldap_authorities_populator::LdapAuthoritiesPopulator,
                ldap_granted_authority::LdapGrantedAuthority,
                ldap_user_details_service::LdapUserDetailsService,
            },
        },
    };

    use super::LdapAuthenticationProvider;

    struct StubAuthenticator;

    #[async_trait]
    impl LdapAuthenticator for StubAuthenticator {
        async fn authenticate(
            &self,
            authentication: &LdapAuthenticationRequest,
        ) -> Result<DirContextOperations, crate::core::authentication_error::AuthenticationError>
        {
            let mut ctx = DirContextOperations::new(format!(
                "uid={},ou=people,dc=example,dc=com",
                authentication.username()
            ));
            ctx.set_attribute("password", [authentication.password().to_string()]);
            Ok(ctx)
        }
    }

    struct StubAuthoritiesPopulator;

    #[async_trait]
    impl LdapAuthoritiesPopulator for StubAuthoritiesPopulator {
        async fn get_granted_authorities(
            &self,
            _user_data: &DirContextOperations,
            username: &str,
        ) -> Vec<Arc<LdapGrantedAuthority>> {
            vec![Arc::new(LdapGrantedAuthority::new(format!(
                "ROLE_{}",
                username.to_uppercase()
            )))]
        }
    }

    struct StubUserSearch {
        users: HashMap<String, DirContextOperations>,
    }

    #[async_trait]
    impl LdapUserSearch for StubUserSearch {
        async fn search_for_user(
            &self,
            username: &str,
        ) -> Result<DirContextOperations, crate::core::userdetails::username_not_found_error::UsernameNotFoundError>
        {
            self.users
                .get(username)
                .cloned()
                .ok_or_else(|| crate::core::userdetails::username_not_found_error::UsernameNotFoundError(username.to_string()))
        }
    }

    #[tokio::test]
    async fn ldap_authentication_provider_loads_authorities() {
        let provider = LdapAuthenticationProvider::with_authorities_populator(
            Arc::new(StubAuthenticator),
            Arc::new(StubAuthoritiesPopulator),
        );

        let request = LdapAuthenticationRequest::new("alice", "secret");
        let authorities = provider.authenticate_request(&request).await.unwrap();
        assert_eq!(authorities, vec!["ROLE_ALICE".to_string()]);
        provider.authenticate(&request).await.unwrap();
    }

    #[tokio::test]
    async fn ldap_user_details_service_maps_user_from_search_result() {
        let mut ctx = DirContextOperations::new("uid=bob,ou=people,dc=example,dc=com");
        ctx.set_attribute("password", ["secret"]);

        let service = LdapUserDetailsService::with_authorities_populator(
            Arc::new(StubUserSearch {
                users: HashMap::from([(String::from("bob"), ctx)]),
            }),
            Arc::new(StubAuthoritiesPopulator),
        );

        let user = service
            .load_user_by_username(String::from("bob"))
            .await
            .unwrap();
        assert_eq!(user.get_username().await, "bob");
        assert_eq!(user.get_password().await, "secret");
        assert_eq!(user.get_authorities().await.len(), 1);
    }
}
