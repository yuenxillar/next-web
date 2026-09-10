use std::{
    fmt,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use next_web_core::traits::http::http_request::HttpRequest;

use crate::{
    core::{authority::GrantedAuthoritiesContainer, GrantedAuthority},
    web::authentication::{Identity, WebAuthenticationDetails},
};

/// This WebAuthenticationDetails implementation allows for storing a list of pre-authenticated Granted Authorities.
#[derive(Clone)]
pub struct PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails {
    authorities: Vec<Arc<dyn GrantedAuthority>>,

    base: WebAuthenticationDetails,
}

impl PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails {
    pub fn new(request: &dyn HttpRequest, authorities: Vec<Arc<dyn GrantedAuthority>>) -> Self {
        Self {
            authorities,
            base: WebAuthenticationDetails::from(request),
        }
    }
}

impl GrantedAuthoritiesContainer for PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails {
    fn granted_authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        &self.authorities
    }
}

impl Identity for PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Deref for PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails {
    type Target = WebAuthenticationDetails;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
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
            "PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails({}; {:?})",
            self.base.to_string(),
            authorities
        )
    }
}
