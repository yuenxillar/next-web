//! The endpoint that issues the tokens of a grant.

use next_web_core::{
    async_trait,
    filter::FilterError,
    http::{HttpMethod, StatusCode},
    traits::{
        filter::{HttpFilter, HttpFilterChain},
        http::{http_request::HttpRequest, http_response::HttpResponse},
        named::Named,
    },
};

use super::{
    super::{
        authorization::{
            consume_authorization_code, OAuth2Authorization, CODE_CHALLENGE_ATTRIBUTE,
            CODE_CHALLENGE_METHOD_ATTRIBUTE, NONCE_ATTRIBUTE, REDIRECT_URI_ATTRIBUTE,
            SCOPES_ATTRIBUTE,
        },
        client::{AuthorizationGrantType, RegisteredClient},
        client_authentication::{authenticate_client, form_parameter},
        context::AuthorizationServerContext,
        error::OAuth2Error,
        token::{
            now, OAuth2AccessToken, OAuth2RefreshToken, OAuth2Token, OAuth2TokenContext,
            OAuth2TokenType,
        },
    },
    support::{error_response, json_response, verify_proof_key},
};
use serde_json::json;

/// Serves the token endpoint of the authorization server.
///
/// The endpoint authenticates the client of the request and then issues the
/// tokens of the grant it names: the authorization code grant, the refresh
/// token grant, and the client credentials grant.
#[derive(Clone)]
pub struct TokenEndpointFilter {
    context: AuthorizationServerContext,
}

impl TokenEndpointFilter {
    /// Creates the filter of the endpoint.
    ///
    /// # Arguments
    ///
    /// * `context` - The context of the authorization server.
    pub fn new(context: AuthorizationServerContext) -> Self {
        Self { context }
    }

    /// Issues the tokens of the request.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    fn issue(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        if request.method() != HttpMethod::POST {
            error_response(
                response,
                &OAuth2Error::invalid_request("The token endpoint supports POST"),
            );
            return Ok(());
        }

        match self.tokens(request, response) {
            Ok(()) => {}
            Err(error) => error_response(response, &error),
        }

        Ok(())
    }

    /// Issues the tokens of the grant of the request.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    fn tokens(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), OAuth2Error> {
        let registered_client = authenticate_client(request, &self.context)?;

        let grant_type = form_parameter(request, "grant_type")
            .ok_or_else(|| OAuth2Error::invalid_request("The grant_type parameter is required"))?;
        let grant_type = AuthorizationGrantType::from_value(&grant_type).ok_or_else(|| {
            OAuth2Error::unsupported_grant_type(format!("The {grant_type} grant is not supported"))
        })?;
        if !registered_client.is_authorized_grant_type(grant_type) {
            return Err(OAuth2Error::unauthorized_client(format!(
                "The client is not authorized for the {} grant",
                grant_type.value()
            )));
        }

        match grant_type {
            AuthorizationGrantType::AuthorizationCode => {
                self.authorization_code(request, response, &registered_client)
            }
            AuthorizationGrantType::RefreshToken => {
                self.refresh_token(request, response, &registered_client)
            }
            AuthorizationGrantType::ClientCredentials => {
                self.client_credentials(request, response, &registered_client)
            }
            AuthorizationGrantType::DeviceCode => {
                self.device_code(request, response, &registered_client)
            }
        }
    }

    /// Issues the tokens of the device authorization grant.
    ///
    /// The device polls the endpoint with its device code. A poll that arrives
    /// before the resource owner answered reports `authorization_pending`, a
    /// poll that arrives too soon reports `slow_down`, and a request the
    /// resource owner denied or that expired is refused. The tokens are issued
    /// once the resource owner approved the request.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    /// * `registered_client` - The authenticated client.
    fn device_code(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        registered_client: &RegisteredClient,
    ) -> Result<(), OAuth2Error> {
        use crate::oauth2::server::authorization::device::DeviceAuthorizationStatus;

        let device_code = form_parameter(request, "device_code")
            .ok_or_else(|| OAuth2Error::invalid_request("The device_code parameter is required"))?;

        let mut device_authorization = self
            .context
            .device_authorizations()
            .find_by_device_code(&device_code)
            .ok_or_else(|| OAuth2Error::invalid_grant("The device code is invalid"))?;
        if device_authorization.registered_client_id() != registered_client.id_value() {
            return Err(OAuth2Error::invalid_grant(
                "The device code was issued to another client",
            ));
        }

        let now = now();
        if device_authorization.is_expired(now) {
            return Err(OAuth2Error::expired_token("The device code is expired"));
        }

        match device_authorization.status() {
            DeviceAuthorizationStatus::Pending => {
                // A device that polls again before the interval of the
                // authorization passed is told to slow down. The check applies
                // while the request is pending, because an approved request is
                // answered with the tokens as soon as it is polled.
                let polled_too_soon = device_authorization
                    .last_polled_at()
                    .is_some_and(|last_polled_at| {
                        now.saturating_sub(last_polled_at) < device_authorization.interval()
                    });
                device_authorization.record_poll(now);
                self.context
                    .device_authorizations()
                    .save(device_authorization);
                if polled_too_soon {
                    Err(OAuth2Error::slow_down("The device polls too quickly"))
                } else {
                    Err(OAuth2Error::authorization_pending(
                        "The resource owner has not answered the request yet",
                    ))
                }
            }
            DeviceAuthorizationStatus::Denied => {
                device_authorization.record_poll(now);
                self.context
                    .device_authorizations()
                    .save(device_authorization);
                Err(OAuth2Error::access_denied(
                    "The resource owner denied the request",
                ))
            }
            DeviceAuthorizationStatus::Approved => {
                device_authorization.record_poll(now);
                let principal_name = device_authorization
                    .principal_name()
                    .unwrap_or(registered_client.client_id())
                    .to_string();
                let scopes = device_authorization.scopes().to_vec();
                self.context
                    .device_authorizations()
                    .save(device_authorization);

                let (access_token, refresh_token) = self.generate_tokens(
                    registered_client,
                    None,
                    Some(&principal_name),
                    &scopes,
                )?;

                let mut authorization = OAuth2Authorization::new(
                    next_web_core::Uuid::new_v4().to_string(),
                    registered_client.id_value(),
                    principal_name.clone(),
                )
                .set_grant_type(AuthorizationGrantType::DeviceCode)
                .set_attribute(SCOPES_ATTRIBUTE, scopes.join(" "))
                .access_token(access_token.clone());
                if let Some(refresh_token) = refresh_token.as_ref() {
                    authorization.set_refresh_token(refresh_token.clone());
                }
                self.context.authorizations().save(authorization);

                let id_token = self.id_token(
                    registered_client,
                    &principal_name,
                    &scopes,
                    None,
                    &access_token,
                )?;
                write_token_response(
                    response,
                    &access_token,
                    refresh_token.as_ref(),
                    &scopes,
                    id_token.as_deref(),
                );
                Ok(())
            }
        }
    }

    /// Issues the tokens of the authorization code grant.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    /// * `registered_client` - The authenticated client.
    fn authorization_code(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        registered_client: &RegisteredClient,
    ) -> Result<(), OAuth2Error> {
        let code = form_parameter(request, "code")
            .ok_or_else(|| OAuth2Error::invalid_request("The code parameter is required"))?;

        let mut authorization = self
            .context
            .authorizations()
            .find_by_token(&code, Some(OAuth2TokenType::AuthorizationCode))
            .ok_or_else(|| OAuth2Error::invalid_grant("The authorization code is invalid"))?;
        if authorization.registered_client_id() != registered_client.id_value() {
            return Err(OAuth2Error::invalid_grant(
                "The authorization code was issued to another client",
            ));
        }
        consume_authorization_code(
            &mut authorization,
            &code,
            self.context.authorization_code_time_to_live(),
        )
        .map_err(OAuth2Error::invalid_grant)?;

        if let Some(registered_redirect_uri) = authorization.attribute(REDIRECT_URI_ATTRIBUTE) {
            match form_parameter(request, "redirect_uri") {
                Some(redirect_uri) if redirect_uri == registered_redirect_uri => {}
                Some(_) => {
                    return Err(OAuth2Error::invalid_grant(
                        "The redirect_uri does not match the authorization request",
                    ));
                }
                None => {
                    return Err(OAuth2Error::invalid_grant(
                        "The redirect_uri parameter is required",
                    ));
                }
            }
        }

        if let Some(code_challenge) = authorization.attribute(CODE_CHALLENGE_ATTRIBUTE) {
            let code_verifier = form_parameter(request, "code_verifier").ok_or_else(|| {
                OAuth2Error::invalid_grant("The code_verifier parameter is required")
            })?;
            let code_challenge_method = authorization
                .attribute(CODE_CHALLENGE_METHOD_ATTRIBUTE)
                .unwrap_or("plain");
            if !verify_proof_key(code_challenge, code_challenge_method, &code_verifier) {
                return Err(OAuth2Error::invalid_grant(
                    "The code_verifier does not match the code_challenge",
                ));
            }
        }

        let scopes = authorization.scopes();
        let (access_token, refresh_token) = self.generate_tokens(
            registered_client,
            Some(authorization.id()),
            Some(authorization.principal_name()),
            &scopes,
        )?;

        let principal_name = authorization.principal_name().to_string();
        let nonce = authorization
            .attribute(NONCE_ATTRIBUTE)
            .map(ToString::to_string);

        authorization.set_access_token(access_token.clone());
        if let Some(refresh_token) = refresh_token.as_ref() {
            authorization.set_refresh_token(refresh_token.clone());
        }
        self.context.authorizations().save(authorization);

        let id_token = self.id_token(
            registered_client,
            &principal_name,
            &scopes,
            nonce.as_deref(),
            &access_token,
        )?;
        write_token_response(
            response,
            &access_token,
            refresh_token.as_ref(),
            &scopes,
            id_token.as_deref(),
        );
        Ok(())
    }

    /// Issues the tokens of the refresh token grant.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    /// * `registered_client` - The authenticated client.
    fn refresh_token(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        registered_client: &RegisteredClient,
    ) -> Result<(), OAuth2Error> {
        let refresh_token = form_parameter(request, "refresh_token").ok_or_else(|| {
            OAuth2Error::invalid_request("The refresh_token parameter is required")
        })?;

        let mut authorization = self
            .context
            .authorizations()
            .find_by_token(&refresh_token, Some(OAuth2TokenType::RefreshToken))
            .ok_or_else(|| OAuth2Error::invalid_grant("The refresh token is invalid"))?;
        if authorization.registered_client_id() != registered_client.id_value() {
            return Err(OAuth2Error::invalid_grant(
                "The refresh token was issued to another client",
            ));
        }
        if !authorization.is_refresh_token_active(now()) {
            return Err(OAuth2Error::invalid_grant("The refresh token is expired"));
        }

        let granted_scopes = authorization.scopes();
        let scopes = match form_parameter(request, "scope") {
            Some(scope) => {
                let requested = scope
                    .split_whitespace()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                for scope in &requested {
                    if !granted_scopes.contains(scope) {
                        return Err(OAuth2Error::invalid_scope(format!(
                            "The {scope} scope was not granted"
                        )));
                    }
                }
                requested
            }
            None => granted_scopes.clone(),
        };

        let (access_token, rotated_refresh_token) = self.generate_tokens(
            registered_client,
            Some(authorization.id()),
            Some(authorization.principal_name()),
            &scopes,
        )?;
        let refresh_token = if registered_client.token_settings().reuse_refresh_tokens() {
            authorization.refresh_token_value().cloned()
        } else {
            rotated_refresh_token.clone()
        };

        let principal_name = authorization.principal_name().to_string();
        let nonce = authorization
            .attribute(NONCE_ATTRIBUTE)
            .map(ToString::to_string);

        authorization.set_access_token(access_token.clone());
        if let Some(rotated_refresh_token) = rotated_refresh_token {
            if !registered_client.token_settings().reuse_refresh_tokens() {
                authorization.set_refresh_token(rotated_refresh_token);
            }
        }
        self.context.authorizations().save(authorization);

        let id_token = self.id_token(
            registered_client,
            &principal_name,
            &scopes,
            nonce.as_deref(),
            &access_token,
        )?;
        write_token_response(
            response,
            &access_token,
            refresh_token.as_ref(),
            &scopes,
            id_token.as_deref(),
        );
        Ok(())
    }

    /// Issues the token of the client credentials grant.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    /// * `registered_client` - The authenticated client.
    fn client_credentials(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        registered_client: &RegisteredClient,
    ) -> Result<(), OAuth2Error> {
        let scopes = match form_parameter(request, "scope") {
            Some(scope) => scope
                .split_whitespace()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            None => registered_client.scopes().to_vec(),
        };
        for scope in &scopes {
            if !registered_client.has_scope(scope) {
                return Err(OAuth2Error::invalid_scope(format!(
                    "The {scope} scope is not registered for the client"
                )));
            }
        }

        let (access_token, _) = self.generate_tokens(registered_client, None, None, &scopes)?;
        let authorization = OAuth2Authorization::new(
            next_web_core::Uuid::new_v4().to_string(),
            registered_client.id_value(),
            registered_client.client_id(),
        )
        .set_grant_type(AuthorizationGrantType::ClientCredentials)
        .set_attribute(SCOPES_ATTRIBUTE, scopes.join(" "))
        .access_token(access_token.clone());
        self.context.authorizations().save(authorization);

        write_token_response(response, &access_token, None, &scopes, None);
        Ok(())
    }

    /// Generates the access token, and the refresh token of the client.
    ///
    /// A refresh token is generated only when the client is allowed to use the
    /// refresh token grant.
    ///
    /// # Arguments
    ///
    /// * `registered_client` - The client the tokens are issued to.
    /// * `authorization_id` - The identifier of the authorization.
    /// * `principal_name` - The name of the principal the tokens are issued
    ///   for.
    /// * `scopes` - The scopes of the tokens.
    fn generate_tokens(
        &self,
        registered_client: &RegisteredClient,
        authorization_id: Option<&str>,
        principal_name: Option<&str>,
        scopes: &[String],
    ) -> Result<(OAuth2AccessToken, Option<OAuth2RefreshToken>), OAuth2Error> {
        let mut context = OAuth2TokenContext::new(
            OAuth2TokenType::AccessToken,
            registered_client.clone(),
            self.context.issuer(),
        )
        .set_scopes(scopes.to_vec());
        if let Some(authorization_id) = authorization_id {
            context = context.set_authorization_id(authorization_id);
        }
        if let Some(principal_name) = principal_name {
            context = context.set_principal_name(principal_name);
        }

        let access_token = match self.context.token_generator().generate(&context)? {
            Some(OAuth2Token::Access(access_token)) => access_token,
            _ => {
                return Err(OAuth2Error::server_error(
                    "The server did not produce an access token",
                ));
            }
        };

        let refresh_token =
            if registered_client.is_authorized_grant_type(AuthorizationGrantType::RefreshToken) {
                let mut context = OAuth2TokenContext::new(
                    OAuth2TokenType::RefreshToken,
                    registered_client.clone(),
                    self.context.issuer(),
                )
                .set_scopes(scopes.to_vec());
                if let Some(authorization_id) = authorization_id {
                    context = context.set_authorization_id(authorization_id);
                }

                match self.context.token_generator().generate(&context)? {
                    Some(OAuth2Token::Refresh(refresh_token)) => Some(refresh_token),
                    _ => None,
                }
            } else {
                None
            };

        Ok((access_token, refresh_token))
    }

    /// Returns the identifier token of a grant that asks for one.
    ///
    /// An OpenID Connect client asks for the `openid` scope, and the identifier
    /// token it receives names the resource owner, the client, and the nonce of
    /// the authorization request.
    ///
    /// # Arguments
    ///
    /// * `registered_client` - The client the grant was issued to.
    /// * `principal_name` - The name of the resource owner.
    /// * `scopes` - The scopes of the grant.
    /// * `nonce` - The nonce of the authorization request, if one was sent.
    /// * `access_token` - The access token of the grant.
    fn id_token(
        &self,
        registered_client: &RegisteredClient,
        principal_name: &str,
        scopes: &[String],
        nonce: Option<&str>,
        access_token: &OAuth2AccessToken,
    ) -> Result<Option<String>, OAuth2Error> {
        if !scopes.iter().any(|scope| scope == "openid") {
            return Ok(None);
        }

        let issuer = self.context.issuer();
        let mut claims = json!({
            "iss": issuer,
            "sub": principal_name,
            "aud": registered_client.client_id(),
            "iat": access_token.issued_at(),
            "exp": access_token.expires_at(),
        });
        if let Some(nonce) = nonce {
            claims["nonce"] = json!(nonce);
        }

        self.context.jwt_signer().sign(&claims).map(Some)
    }
}

#[async_trait]
impl HttpFilter for TokenEndpointFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.path() != self.context.settings().token_endpoint() {
            return filter_chain.do_filter(request, response).await;
        }

        self.issue(request, response)
    }
}

impl Named for TokenEndpointFilter {
    fn name(&self) -> &str {
        "TokenEndpointFilter"
    }
}

/// Writes the token response of a grant.
///
/// # Arguments
///
/// * `response` - The response of the endpoint.
/// * `access_token` - The access token of the grant.
/// * `refresh_token` - The refresh token of the grant, if one was issued.
/// * `scopes` - The scopes of the grant.
pub(crate) fn write_token_response(
    response: &mut dyn HttpResponse,
    access_token: &OAuth2AccessToken,
    refresh_token: Option<&OAuth2RefreshToken>,
    scopes: &[String],
    id_token: Option<&str>,
) {
    let mut body = json!({
        "access_token": access_token.token_value(),
        "token_type": access_token.token_type(),
        "expires_in": access_token.expires_at().saturating_sub(now()),
    });
    if let Some(refresh_token) = refresh_token {
        body["refresh_token"] = json!(refresh_token.token_value());
    }
    if !scopes.is_empty() {
        body["scope"] = json!(scopes.join(" "));
    }
    if let Some(id_token) = id_token {
        body["id_token"] = json!(id_token);
    }

    json_response(response, StatusCode::OK, &body);
}
