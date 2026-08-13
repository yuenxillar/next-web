use std::{fmt, sync::Arc};

use next_web_core::traits::http::http_request::HttpRequest;

use crate::core::{granted_authorities_container::GrantedAuthoritiesContainer, GrantedAuthority};

#[derive(Clone, Default)]
pub struct PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails {
    remote_address: Option<String>,
    session_id: Option<String>,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
}

impl PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails {
    pub fn new(request: &dyn HttpRequest, authorities: Vec<Arc<dyn GrantedAuthority>>) -> Self {
        let remote_address = request.header("x-forwarded-for").map(ToOwned::to_owned);
        let session_id = request.header("x-session-id").map(ToOwned::to_owned);

        Self {
            remote_address,
            session_id,
            authorities,
        }
    }

    pub fn remote_address(&self) -> Option<&str> {
        self.remote_address.as_deref()
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }
}

impl GrantedAuthoritiesContainer for PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails {
    fn granted_authorities(&self) -> Vec<Arc<dyn GrantedAuthority>> {
        self.authorities.clone()
    }
}

impl fmt::Display for PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let authorities = self
            .authorities
            .iter()
            .filter_map(|authority| authority.authority())
            .collect::<Vec<_>>();

        write!(
            f,
            "PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails(remote_address={:?}, session_id={:?}, authorities={:?})",
            self.remote_address, self.session_id, authorities
        )
    }
}
