//! The user information loaded from the OpenID Connect user-info endpoint.

mod oidc_user_authority;

pub use oidc_user_authority::OidcUserAuthority;

use std::sync::Arc;

use crate::{
    core::{AuthenticationError, AuthenticationErrorKind, GrantedAuthority},
    oauth2::{
        client::userinfo::user_of, ClientRegistration, OAuth2AccessTokenResponse,
        OAuth2UserService, OidcIdToken,
    },
};

use next_web_core::async_trait;

/// The request the OpenID Connect user information is loaded with.
#[derive(Clone)]
pub struct OidcUserRequest {
    client_registration: ClientRegistration,
    access_token_response: OAuth2AccessTokenResponse,
    id_token: Option<OidcIdToken>,
}

impl OidcUserRequest {
    /// Creates the request of a grant.
    ///
    /// # Arguments
    ///
    /// * `client_registration` - The client the grant was issued to.
    /// * `access_token_response` - The response of the token endpoint.
    pub fn new(
        client_registration: ClientRegistration,
        access_token_response: OAuth2AccessTokenResponse,
    ) -> Self {
        Self {
            client_registration,
            access_token_response,
            id_token: None,
        }
    }

    /// Sets the identifier token of the grant that was validated.
    ///
    /// The validation of the token is what lets the user of the grant be read
    /// from the claims the token names, so the token is handed to the request
    /// that loads the user.
    ///
    /// # Arguments
    ///
    /// * `id_token` - The validated identifier token of the grant.
    pub fn set_id_token(mut self, id_token: OidcIdToken) -> Self {
        self.id_token = Some(id_token);
        self
    }

    /// Returns the validated identifier token of the grant, if one was
    /// validated.
    pub fn id_token(&self) -> Option<&OidcIdToken> {
        self.id_token.as_ref()
    }

    /// Returns the client the grant was issued to.
    pub fn client_registration(&self) -> &ClientRegistration {
        &self.client_registration
    }

    /// Returns the response of the token endpoint.
    pub fn access_token_response(&self) -> &OAuth2AccessTokenResponse {
        &self.access_token_response
    }
}

/// The user of an OpenID Connect grant.
#[derive(Clone, Default)]
pub struct OidcUser {
    name: String,
    issuer: Option<String>,
    claims: serde_json::Value,
    authorities: Vec<Arc<dyn GrantedAuthority>>,
}

impl OidcUser {
    /// Creates the user of an OpenID Connect grant.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the user, which is the subject the provider
    ///   names.
    /// * `issuer` - The issuer of the user, if the grant names one.
    pub fn new(name: impl Into<String>, issuer: Option<String>) -> Self {
        Self {
            name: name.into(),
            issuer,
            claims: serde_json::Value::Null,
            authorities: Vec::new(),
        }
    }

    /// Sets the claims of the user.
    ///
    /// # Arguments
    ///
    /// * `claims` - The claims of the user.
    pub fn set_claims(mut self, claims: serde_json::Value) -> Self {
        self.claims = claims;
        self
    }

    /// Sets the authorities of the user.
    ///
    /// # Arguments
    ///
    /// * `authorities` - The authorities of the user.
    pub fn set_authorities(mut self, authorities: Vec<Arc<dyn GrantedAuthority>>) -> Self {
        self.authorities = authorities;
        self
    }

    /// Returns the name of the user.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the issuer of the user, if the identifier token names one.
    pub fn issuer(&self) -> Option<&str> {
        self.issuer.as_deref()
    }

    /// Returns the claims of the user.
    ///
    /// The claims are the claims of the identifier token of the grant, with the
    /// claims of the user-info endpoint the provider served added to them.
    pub fn claims(&self) -> &serde_json::Value {
        &self.claims
    }

    /// Returns the authorities of the user.
    pub fn authorities(&self) -> &[Arc<dyn GrantedAuthority>] {
        &self.authorities
    }
}

impl std::fmt::Debug for OidcUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OidcUser")
            .field("name", &self.name)
            .field("issuer", &self.issuer)
            .field("claims", &self.claims)
            .field("authorities", &self.authorities.len())
            .finish()
    }
}

/// An [`OAuth2UserService`] that reads the user of an OpenID Connect grant.
///
/// The user is read from the user-info endpoint of the provider, like the user
/// of a plain grant, and the identifier token of the grant names the issuer of
/// the user.
#[derive(Clone, Default)]
pub struct OidcUserService;

#[async_trait]
impl OAuth2UserService<OidcUserRequest, OidcUser> for OidcUserService {
    async fn load_user(&self, request: &OidcUserRequest) -> Result<OidcUser, AuthenticationError> {
        let access_token = request.access_token_response().access_token();
        if access_token.trim().is_empty() {
            return Err(AuthenticationError::with_kind(
                "The grant has no access token",
                AuthenticationErrorKind::BadCredentials,
            ));
        }

        // The identifier token of the grant was validated before the user is
        // loaded, so the issuer and the subject it names can be used. A grant
        // that was not validated is read the way it was before.
        let issuer = request
            .id_token()
            .map(|id_token| id_token.issuer().to_string())
            .or_else(|| {
                request
                    .access_token_response()
                    .id_token()
                    .and_then(|id_token| claim_of(id_token, "iss"))
            });
        let subject = request
            .id_token()
            .map(|id_token| id_token.subject().to_string())
            .or_else(|| {
                request
                    .access_token_response()
                    .id_token()
                    .and_then(|id_token| claim_of(id_token, "sub"))
            });

        let mut user_info = None;
        let user = match request.client_registration().user_info_uri() {
            Some(user_info_uri) => {
                let document = fetch_user_info(user_info_uri, access_token).await?;
                // The user of the grant is the subject the identifier token
                // names, so a user-info endpoint that answers with another
                // subject is refused.
                if let (Some(subject), Some(user_subject)) = (
                    subject.as_deref(),
                    document.get("sub").and_then(serde_json::Value::as_str),
                ) {
                    if subject != user_subject {
                        return Err(AuthenticationError::with_kind(
                            "The subject of the user info endpoint is not the subject of the \
                             identifier token",
                            AuthenticationErrorKind::BadCredentials,
                        ));
                    }
                }
                let user = user_of(&document)?;
                user_info = Some(document);
                user
            }
            // A provider that does not serve a user-info endpoint names the
            // user of the grant by the identifier token alone.
            None => crate::oauth2::OAuth2User::new(
                subject.unwrap_or_else(|| access_token.to_string()),
                Default::default(),
            ),
        };

        // The user of the grant is described by the identifier token of the
        // grant: its claims are the claims of the user, and the authority of
        // the user carries the token so that a rule can look the claims up.
        let id_token = request.id_token().cloned();
        let claims = claims_of(id_token.as_ref(), user_info.as_ref());
        let authorities = authorities_of(id_token.as_ref(), user_info, &claims);

        Ok(OidcUser::new(user.name().to_string(), issuer)
            .set_claims(claims)
            .set_authorities(authorities))
    }
}

/// Returns the claims of the user of a grant.
///
/// The claims are the claims of the identifier token of the grant, and a claim
/// that the user information names is added to them.
///
/// # Arguments
///
/// * `id_token` - The validated identifier token of the grant, if one was
///   validated.
/// * `user_info` - The document of the user-info endpoint, if one was served.
fn claims_of(
    id_token: Option<&OidcIdToken>,
    user_info: Option<&serde_json::Value>,
) -> serde_json::Value {
    let mut claims = id_token
        .map(|id_token| id_token.claims().clone())
        .unwrap_or(serde_json::Value::Null);
    if let (Some(claims), Some(user_info)) = (claims.as_object_mut(), user_info) {
        if let Some(user_info) = user_info.as_object() {
            for (name, value) in user_info {
                claims.entry(name.clone()).or_insert_with(|| value.clone());
            }
        }
    }

    claims
}

/// Returns the authorities of the user of a grant.
///
/// The authority of an OpenID Connect user carries the identifier token of the
/// grant, and the scopes the token names become authorities of their own.
///
/// # Arguments
///
/// * `id_token` - The validated identifier token of the grant, if one was
///   validated.
/// * `user_info` - The document of the user-info endpoint, if one was served.
/// * `claims` - The claims of the user.
fn authorities_of(
    id_token: Option<&OidcIdToken>,
    user_info: Option<serde_json::Value>,
    claims: &serde_json::Value,
) -> Vec<Arc<dyn GrantedAuthority>> {
    let mut authorities: Vec<Arc<dyn GrantedAuthority>> = Vec::new();
    if let Some(id_token) = id_token {
        authorities.push(Arc::new(OidcUserAuthority::new(
            id_token.clone(),
            user_info,
        )));
    }
    authorities.extend(scopes_of(claims).into_iter().map(|scope| {
        let authority = if scope.starts_with("SCOPE_") {
            scope
        } else {
            format!("SCOPE_{scope}")
        };
        Arc::new(crate::core::authority::SimpleGrantedAuthority::new(
            authority,
        )) as Arc<dyn GrantedAuthority>
    }));

    authorities
}

/// Returns the scopes a document names.
///
/// # Arguments
///
/// * `claims` - The claims of the user.
fn scopes_of(claims: &serde_json::Value) -> Vec<String> {
    if let Some(scope) = claims.get("scope").and_then(serde_json::Value::as_str) {
        return scope
            .split_whitespace()
            .filter(|scope| !scope.is_empty())
            .map(ToString::to_string)
            .collect();
    }

    claims
        .get("scp")
        .and_then(serde_json::Value::as_array)
        .map(|scopes| {
            scopes
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Reads a claim of an identifier token without verifying its signature.
///
/// # Arguments
///
/// * `id_token` - The identifier token of the grant.
/// * `name` - The name of the claim.
fn claim_of(id_token: &str, name: &str) -> Option<String> {
    let payload = id_token.split('.').nth(1)?;
    let claims: serde_json::Value =
        serde_json::from_slice(&crate::oauth2::jose::base64_url_decode(payload)?).ok()?;

    claims
        .get(name)
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
}

/// Reads the document of the user-info endpoint of the provider.
///
/// # Arguments
///
/// * `user_info_uri` - The URI of the endpoint.
/// * `access_token` - The access token of the grant.
async fn fetch_user_info(
    user_info_uri: &str,
    access_token: &str,
) -> Result<serde_json::Value, AuthenticationError> {
    let response = reqwest::Client::new()
        .get(user_info_uri)
        .bearer_auth(access_token)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|error| {
            AuthenticationError::with_kind(
                format!("The user info endpoint is not reached: {error}"),
                AuthenticationErrorKind::AuthenticationService,
            )
        })?;
    let status = response.status();
    let document = response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| {
            AuthenticationError::with_kind(
                format!("The response of the user info endpoint is not JSON: {error}"),
                AuthenticationErrorKind::AuthenticationService,
            )
        })?;

    if !status.is_success() {
        return Err(AuthenticationError::with_kind(
            format!("The user info endpoint reported {status}"),
            AuthenticationErrorKind::BadCredentials,
        ));
    }

    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::{authorities_of, claim_of, claims_of, OidcUserRequest, OidcUserService};
    use crate::{
        core::GrantedAuthority,
        oauth2::{
            jose::base64_url, ClientRegistration, OAuth2AccessTokenResponse, OAuth2UserService,
            OidcIdToken,
        },
    };

    #[test]
    fn a_claim_of_an_identifier_token_is_read() {
        let claims = serde_json::json!({ "iss": "https://server.example", "sub": "user" });
        let token = format!(
            "{}.{}.signature",
            base64_url(b"{\"alg\":\"RS256\"}"),
            base64_url(claims.to_string().as_bytes())
        );

        assert_eq!(
            claim_of(&token, "iss").as_deref(),
            Some("https://server.example")
        );
        assert_eq!(claim_of(&token, "sub").as_deref(), Some("user"));
        assert_eq!(claim_of(&token, "aud"), None);
    }

    #[test]
    fn a_token_of_another_kind_has_no_claims_to_read() {
        assert_eq!(claim_of("not-a-token", "iss"), None);
    }

    #[tokio::test]
    async fn a_user_info_response_of_another_subject_is_refused() {
        // The endpoint answers with a subject of its own, which is not the
        // subject the identifier token of the grant names.
        async fn user_info() -> axum::Json<serde_json::Value> {
            axum::Json(serde_json::json!({ "sub": "someone-else" }))
        }

        let app = axum::Router::new().route("/userinfo", axum::routing::get(user_info));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("the endpoint listens");
        let address = listener.local_addr().expect("the listener has an address");
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let registration =
            ClientRegistration::new("demo", "Demo", "https://provider.example/authorize")
                .set_client_id("demo-client")
                .set_user_info_uri(format!("http://{address}/userinfo"));
        let id_token = OidcIdToken::new(
            "https://provider.example",
            "user",
            vec!["demo-client".to_string()],
            serde_json::json!({ "sub": "user" }),
        );
        let request = OidcUserRequest::new(
            registration,
            OAuth2AccessTokenResponse::new("the-access-token"),
        )
        .set_id_token(id_token);

        let error = OidcUserService
            .load_user(&request)
            .await
            .expect_err("the subject of the user info endpoint is not the subject of the token");

        assert!(error.message().contains("not the subject"));
    }

    #[test]
    fn the_scopes_of_the_identifier_token_become_authorities_of_the_user() {
        let id_token = OidcIdToken::new(
            "https://provider.example",
            "user",
            vec!["demo-client".to_string()],
            serde_json::json!({ "sub": "user", "scope": "openid read" }),
        );

        let authorities = authorities_of(Some(&id_token), None, id_token.claims());
        let values: Vec<String> = authorities
            .iter()
            .filter_map(|authority| authority.authority().map(ToString::to_string))
            .collect();

        assert!(values.contains(&"OIDC_USER".to_string()), "{values:?}");
        assert!(values.contains(&"SCOPE_openid".to_string()), "{values:?}");
        assert!(values.contains(&"SCOPE_read".to_string()), "{values:?}");
    }

    #[test]
    fn the_claims_of_the_user_are_merged_with_the_user_information() {
        let id_token = OidcIdToken::new(
            "https://provider.example",
            "user",
            vec!["demo-client".to_string()],
            serde_json::json!({ "sub": "user" }),
        );

        let claims = claims_of(
            Some(&id_token),
            Some(&serde_json::json!({ "email": "user@example.com" })),
        );

        assert_eq!(claims["sub"], "user");
        assert_eq!(claims["email"], "user@example.com");
    }
}
