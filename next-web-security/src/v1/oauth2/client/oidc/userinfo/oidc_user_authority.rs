//! The authority of a user that was authenticated with an OpenID Connect
//! identifier token.

use std::fmt;

use serde_json::Value;

use crate::{core::GrantedAuthority, oauth2::OidcIdToken};

/// The authority of a user of an OpenID Connect grant.
///
/// The authority carries the identifier token of the grant and, when the
/// provider served one, the user information of the user. A rule of an
/// application can look the claims of the token up through the authority, which
/// is what an expression such as `hasAuthority('OIDC_USER')` and an
/// authorization manager that understands this authority are written with.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OidcUserAuthority {
    id_token: OidcIdToken,
    user_info: Option<Value>,
}

impl OidcUserAuthority {
    /// The value of the authority.
    pub const AUTHORITY: &'static str = "OIDC_USER";

    /// Creates the authority of a grant that named a user.
    ///
    /// # Arguments
    ///
    /// * `id_token` - The validated identifier token of the grant.
    /// * `user_info` - The document of the user-info endpoint, when the
    ///   provider served one.
    pub fn new(id_token: OidcIdToken, user_info: Option<Value>) -> Self {
        Self {
            id_token,
            user_info,
        }
    }

    /// Creates the authority of a grant that has no user information.
    ///
    /// # Arguments
    ///
    /// * `id_token` - The validated identifier token of the grant.
    pub fn with_id_token(id_token: OidcIdToken) -> Self {
        Self::new(id_token, None)
    }

    /// Returns the identifier token of the grant.
    pub fn id_token(&self) -> &OidcIdToken {
        &self.id_token
    }

    /// Returns the document of the user-info endpoint, if the grant has one.
    pub fn user_info(&self) -> Option<&Value> {
        self.user_info.as_ref()
    }

    /// Returns the claims of the user.
    ///
    /// The claims of the identifier token are the claims of the user, and a
    /// claim the user information names is added to them.
    pub fn claims(&self) -> Value {
        let mut claims = self.id_token.claims().clone();
        if let (Some(claims), Some(user_info)) = (claims.as_object_mut(), self.user_info.as_ref()) {
            if let Some(user_info) = user_info.as_object() {
                for (name, value) in user_info {
                    claims.entry(name.clone()).or_insert_with(|| value.clone());
                }
            }
        }

        claims
    }
}

impl GrantedAuthority for OidcUserAuthority {
    fn authority(&self) -> Option<&str> {
        Some(Self::AUTHORITY)
    }
}

impl fmt::Display for OidcUserAuthority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}/{})",
            Self::AUTHORITY,
            self.id_token.issuer(),
            self.id_token.subject()
        )
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::OidcUserAuthority;
    use crate::{core::GrantedAuthority, oauth2::OidcIdToken};

    /// Returns the identifier token of a test.
    fn id_token() -> OidcIdToken {
        OidcIdToken::new(
            "https://provider.example",
            "user",
            vec!["demo-client".to_string()],
            json!({ "sub": "user", "iss": "https://provider.example" }),
        )
    }

    #[test]
    fn the_authority_of_an_openid_user_is_named_oidc_user() {
        let authority = OidcUserAuthority::with_id_token(id_token());

        assert_eq!(authority.authority(), Some("OIDC_USER"));
        assert_eq!(authority.id_token().subject(), "user");
    }

    #[test]
    fn the_claims_of_the_user_are_the_claims_of_the_token_and_the_user_information() {
        let authority = OidcUserAuthority::new(
            id_token(),
            Some(json!({ "email": "user@example.com", "sub": "user" })),
        );

        let claims = authority.claims();

        assert_eq!(claims["sub"], "user");
        assert_eq!(claims["iss"], "https://provider.example");
        assert_eq!(claims["email"], "user@example.com");
    }
}
