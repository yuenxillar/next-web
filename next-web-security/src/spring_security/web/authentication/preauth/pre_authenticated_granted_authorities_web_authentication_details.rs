use std::{fmt, sync::Arc};

use axum::extract::Request;

use crate::core::{
    granted_authorities_container::GrantedAuthoritiesContainer, granted_authority::GrantedAuthority,
};

#[derive(Clone, Default)]
pub struct PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails {
    remote_address: Option<String>,
    session_id: Option<String>,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
}

impl PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails {
    pub fn new(
        request: &Request,
        authorities: Vec<Arc<dyn GrantedAuthority>>,
    ) -> Self {
        let remote_address = request
            .headers()
            .get("x-forwarded-for")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        let session_id = request
            .headers()
            .get("x-session-id")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);

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
            .filter_map(|authority| futures::executor::block_on(authority.get_authority()))
            .collect::<Vec<_>>();

        write!(
            f,
            "PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails(remote_address={:?}, session_id={:?}, authorities={:?})",
            self.remote_address, self.session_id, authorities
        )
    }
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request, http::Request as HttpRequest};

    use crate::core::{authority_utils::AuthorityUtils, granted_authorities_container::GrantedAuthoritiesContainer};

    use super::PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails;

    #[test]
    fn details_expose_granted_authorities_and_debuggable_string() {
        let request = Request::from(
            HttpRequest::builder()
                .header("x-forwarded-for", "127.0.0.1")
                .header("x-session-id", "session-1")
                .body(Body::empty())
                .unwrap(),
        );
        let authorities = AuthorityUtils::create_authority_list(["Role1", "Role2"]);

        let details =
            PreAuthenticatedGrantedAuthoritiesWebAuthenticationDetails::new(&request, authorities);

        let granted = details.granted_authorities();
        assert_eq!(granted.len(), 2);

        let rendered = details.to_string();
        assert!(rendered.contains("Role1"));
        assert!(rendered.contains("Role2"));
    }
}
