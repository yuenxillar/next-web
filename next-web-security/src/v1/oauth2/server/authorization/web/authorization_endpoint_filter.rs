//! The endpoint that grants an authorization code to a client.

use std::{collections::HashMap, sync::Arc};

use next_web_core::{
    any::AnyValue,
    async_trait,
    http::{
        filter::{FilterError, HttpFilter, HttpFilterChain},
        HttpMethod, HttpRequest, HttpResponse,
    },
    Named,
};

use super::{
    super::{
        authorization::{
            OAuth2Authorization, AUTHORIZATION_CODE_ATTRIBUTE, CODE_CHALLENGE_ATTRIBUTE,
            CODE_CHALLENGE_METHOD_ATTRIBUTE, NONCE_ATTRIBUTE, REDIRECT_URI_ATTRIBUTE,
            SCOPES_ATTRIBUTE, STATE_ATTRIBUTE,
        },
        client::{AuthorizationGrantType, RegisteredClient},
        client_authentication::{
            form_parameter, form_parameter_map, PUSHED_AUTHORIZATION_REQUEST_ATTRIBUTE,
        },
        consent::OAuth2AuthorizationConsent,
        context::AuthorizationServerContext,
        error::OAuth2Error,
        token::{now, OAuth2TokenType},
    },
    support::{error_response, resource_owner_required, send_redirect},
};
use crate::{
    authentication::AuthenticationTrustResolverImpl,
    authorization::AuthenticationTrustResolver,
    core::{context::SecurityContextHolder, Authentication},
};

/// Serves the authorization endpoint of the authorization server.
///
/// The endpoint requires the resource owner to be logged in: a resource owner
/// that is not logged in is sent to the login page, and the request is repeated
/// after the login, which is what the error translation filter of the
/// application does. The consent of the resource owner is not asked, so the
/// scopes of the client are granted as they are requested.
#[derive(Clone)]
pub struct AuthorizationEndpointFilter {
    context: AuthorizationServerContext,
    trust_resolver: Arc<dyn AuthenticationTrustResolver>,
}

impl AuthorizationEndpointFilter {
    /// Creates the filter of the endpoint.
    ///
    /// # Arguments
    ///
    /// * `context` - The context of the authorization server.
    pub fn new(context: AuthorizationServerContext) -> Self {
        Self {
            context,
            trust_resolver: Arc::new(AuthenticationTrustResolverImpl::default()),
        }
    }

    /// Returns the resource owner of the request, if one is logged in.
    fn resource_owner(&self) -> Option<Arc<dyn Authentication>> {
        SecurityContextHolder::get_context()
            .get_authentication()
            .filter(|authentication| !self.trust_resolver.is_anonymous(Some(&**authentication)))
    }

    /// Grants the authorization code of the request.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    fn authorize(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    ) -> Result<(), FilterError> {
        // A client whose authorization request was pushed to the pushed
        // authorization request endpoint names the request URI instead of the
        // parameters, so the parameters of the request are read back from the
        // repository before the request is served.
        if let Some(request_uri) = form_parameter(request, "request_uri") {
            if let Err(error) = self.load_pushed_request(request, &request_uri) {
                error_response(response, &error);
                return Ok(());
            }
        }

        // The client is identified before everything else, because the errors
        // that follow are reported to the redirect URI of the client.
        let Some(client_id) = form_parameter(request, "client_id") else {
            error_response(
                response,
                &OAuth2Error::invalid_request("The client_id parameter is required"),
            );
            return Ok(());
        };
        let Some(registered_client) = self
            .context
            .registered_clients()
            .find_by_client_id(&client_id)
        else {
            error_response(
                response,
                &OAuth2Error::invalid_request(format!("Registered client not found: {client_id}")),
            );
            return Ok(());
        };
        let redirect_uri = match self.redirect_uri(request, &registered_client) {
            Ok(redirect_uri) => redirect_uri,
            Err(error) => {
                error_response(response, &error);
                return Ok(());
            }
        };
        let state = form_parameter(request, "state");

        let scopes = match self.scopes(request, &registered_client) {
            Ok(scopes) => scopes,
            Err(error) => {
                return self.redirect_error(
                    request,
                    response,
                    &redirect_uri,
                    error,
                    state.as_deref(),
                )
            }
        };
        let proof_key = match self.proof_key(request, &registered_client) {
            Ok(proof_key) => proof_key,
            Err(error) => {
                return self.redirect_error(
                    request,
                    response,
                    &redirect_uri,
                    error,
                    state.as_deref(),
                )
            }
        };

        let Some(resource_owner) = self.resource_owner() else {
            return Err(resource_owner_required());
        };

        if request.method() == HttpMethod::POST {
            return self.submit_consent(
                request,
                response,
                &registered_client,
                state.as_deref(),
                resource_owner.name().as_ref(),
            );
        }
        if request.method() != HttpMethod::GET {
            error_response(
                response,
                &OAuth2Error::invalid_request("The authorization endpoint supports GET and POST"),
            );
            return Ok(());
        }

        // A deployment may require every authorization request to be pushed to
        // the pushed authorization request endpoint, so that the parameters of
        // the request never travel in the URL of the browser.
        if self
            .context
            .settings()
            .require_pushed_authorization_requests()
            && form_parameter(request, "request_uri").is_none()
        {
            error_response(
                response,
                &OAuth2Error::invalid_request(
                    "The authorization request has to be pushed to the pushed authorization \
                     request endpoint",
                ),
            );
            return Ok(());
        }

        // The resource owner is asked for consent when the client requires it
        // and the scopes of the request are not part of the consent the resource
        // owner granted before.
        if self.requires_consent(&registered_client, resource_owner.name().as_ref(), &scopes) {
            return self.send_consent(
                request,
                response,
                &registered_client,
                &redirect_uri,
                state.as_deref(),
                &resource_owner.name(),
                &scopes,
                proof_key,
                form_parameter(request, "nonce"),
            );
        }

        self.issue_code(
            request,
            response,
            &registered_client,
            &redirect_uri,
            state.as_deref(),
            &resource_owner.name(),
            &scopes,
            proof_key,
            form_parameter(request, "nonce"),
        )
    }

    /// Reads the parameters of a pushed authorization request back into the
    /// request.
    ///
    /// The request URI is consumed, so a pushed authorization request cannot be
    /// used twice, and the parameters it carries are handed to the rest of the
    /// endpoint through the request attribute of the pushed authorization
    /// request.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `request_uri` - The request URI of the client.
    fn load_pushed_request(
        &self,
        request: &mut dyn HttpRequest,
        request_uri: &str,
    ) -> Result<(), OAuth2Error> {
        let pushed = self
            .context
            .pushed_authorization_requests()
            .consume(request_uri)
            .ok_or_else(|| {
                OAuth2Error::invalid_request("The request_uri is unknown or was used already")
            })?;
        if pushed.is_expired(now()) {
            return Err(OAuth2Error::invalid_request("The request_uri is expired"));
        }

        let client_id = form_parameter(request, "client_id")
            .ok_or_else(|| OAuth2Error::invalid_request("The client_id parameter is required"))?;
        let registered_client = self
            .context
            .registered_clients()
            .find_by_client_id(&client_id)
            .ok_or_else(|| {
                OAuth2Error::invalid_request(format!("Registered client not found: {client_id}"))
            })?;
        if pushed.registered_client_id() != registered_client.id_value() {
            return Err(OAuth2Error::invalid_request(
                "The request_uri belongs to another client",
            ));
        }

        let mut parameters = HashMap::new();
        for (name, value) in pushed.parameters() {
            parameters.insert(name.clone(), AnyValue::String(value.clone()));
        }
        parameters.insert("client_id".to_string(), AnyValue::String(client_id));
        request.set_attribute(
            PUSHED_AUTHORIZATION_REQUEST_ATTRIBUTE,
            AnyValue::Map(parameters),
        );

        Ok(())
    }

    /// Returns whether the resource owner has to be asked for consent.
    ///
    /// Consent is asked when the client requires it, when the application
    /// serves a consent page, and when the scopes of the request are not all
    /// part of the consent the resource owner granted before.
    ///
    /// # Arguments
    ///
    /// * `registered_client` - The client of the request.
    /// * `principal_name` - The name of the resource owner.
    /// * `scopes` - The scopes of the request.
    fn requires_consent(
        &self,
        registered_client: &RegisteredClient,
        principal_name: &str,
        scopes: &[String],
    ) -> bool {
        if !registered_client
            .client_settings()
            .require_authorization_consent()
        {
            return false;
        }
        if self.context.consent_page().is_none() {
            // The application does not serve a consent page, so the scopes of
            // the request are granted without asking.
            tracing::debug!(
                "The {} client requires consent, but the authorization server has no consent \
                 page, so the scopes are granted without asking",
                registered_client.client_id()
            );
            return false;
        }

        match self
            .context
            .authorization_consents()
            .find_by_id(registered_client.id_value(), principal_name)
        {
            Some(consent) => !consent.has_scopes(scopes),
            None => !scopes.is_empty(),
        }
    }

    /// Asks the resource owner for consent.
    ///
    /// The authorization request is kept while the resource owner answers, and
    /// the authorization endpoint is asked again by the consent page with the
    /// scopes the resource owner approved.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    /// * `registered_client` - The client of the request.
    /// * `redirect_uri` - The redirect URI of the client.
    /// * `state` - The state of the authorization request.
    /// * `principal_name` - The name of the resource owner.
    /// * `scopes` - The scopes the client asked for.
    /// * `proof_key` - The proof key of the request and its method.
    #[allow(clippy::too_many_arguments)]
    fn send_consent(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        registered_client: &RegisteredClient,
        redirect_uri: &str,
        state: Option<&str>,
        principal_name: &str,
        scopes: &[String],
        proof_key: Option<(String, Option<String>)>,
        nonce: Option<String>,
    ) -> Result<(), FilterError> {
        // The state identifies the authorization request while the resource
        // owner answers, so one is generated when the client did not send one.
        let state = state
            .map(ToString::to_string)
            .unwrap_or_else(|| next_web_core::Uuid::new_v4().to_string());

        let mut authorization = OAuth2Authorization::new(
            next_web_core::Uuid::new_v4().to_string(),
            registered_client.id_value(),
            principal_name,
        )
        .set_grant_type(AuthorizationGrantType::AuthorizationCode)
        .set_attribute(STATE_ATTRIBUTE, state.clone())
        .set_attribute(REDIRECT_URI_ATTRIBUTE, redirect_uri)
        .set_attribute(SCOPES_ATTRIBUTE, scopes.join(" "))
        .set_attribute("authorization_request_issued_at", now().to_string());
        if let Some(nonce) = nonce.as_ref() {
            authorization = authorization.set_attribute(NONCE_ATTRIBUTE, nonce.clone());
        }
        if let Some((code_challenge, code_challenge_method)) = proof_key {
            authorization = authorization.set_attribute(CODE_CHALLENGE_ATTRIBUTE, code_challenge);
            if let Some(code_challenge_method) = code_challenge_method {
                authorization = authorization
                    .set_attribute(CODE_CHALLENGE_METHOD_ATTRIBUTE, code_challenge_method);
            }
        }
        self.context.authorizations().save(authorization);

        let consent_page = self
            .context
            .consent_page()
            .expect("consent is asked only when a page is configured");
        let scopes = scopes.join(" ");
        send_redirect(
            request,
            response,
            consent_page,
            &[
                ("client_id", registered_client.client_id()),
                ("scope", scopes.as_str()),
                ("state", state.as_str()),
            ],
        )
    }

    /// Grants the scopes the resource owner approved.
    ///
    /// The authorization endpoint is asked again by the consent page of the
    /// application with the scopes the resource owner approved, and those
    /// scopes have to be part of the scopes of the authorization request.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    /// * `registered_client` - The client of the request.
    /// * `state` - The state of the authorization request.
    /// * `principal_name` - The name of the resource owner.
    fn submit_consent(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        registered_client: &RegisteredClient,
        state: Option<&str>,
        principal_name: &str,
    ) -> Result<(), FilterError> {
        let Some(state) = state else {
            error_response(
                response,
                &OAuth2Error::invalid_request("The state parameter is required"),
            );
            return Ok(());
        };

        let pending = match self
            .context
            .authorizations()
            .find_by_token(state, Some(OAuth2TokenType::State))
        {
            Some(pending) => pending,
            None => {
                error_response(
                    response,
                    &OAuth2Error::invalid_request("The authorization request is not found"),
                );
                return Ok(());
            }
        };
        if pending.registered_client_id() != registered_client.id_value()
            || pending.principal_name() != principal_name
        {
            error_response(
                response,
                &OAuth2Error::invalid_request(
                    "The authorization request belongs to another client or resource owner",
                ),
            );
            return Ok(());
        }

        let redirect_uri = pending
            .attribute(REDIRECT_URI_ATTRIBUTE)
            .unwrap_or_default()
            .to_string();
        let requested = pending.scopes();
        let approved = approved_scopes(request);
        for scope in &approved {
            if !requested.contains(scope) {
                return self.redirect_error(
                    request,
                    response,
                    &redirect_uri,
                    OAuth2Error::invalid_scope(format!(
                        "The {scope} scope was not part of the authorization request"
                    )),
                    Some(state),
                );
            }
        }
        if approved.is_empty() {
            self.context.authorizations().remove(&pending);
            return self.redirect_error(
                request,
                response,
                &redirect_uri,
                OAuth2Error::access_denied("The resource owner denied the authorization request"),
                Some(state),
            );
        }

        // The scopes the resource owner approved are remembered, so a request
        // whose scopes were approved before is granted without asking again.
        let mut consent =
            OAuth2AuthorizationConsent::with_id(registered_client.id_value(), principal_name);
        consent.grant_scopes(approved.iter().cloned());
        if let Some(existing) = self
            .context
            .authorization_consents()
            .find_by_id(registered_client.id_value(), principal_name)
        {
            consent.grant_scopes(existing.authorities().iter().cloned());
        }
        self.context.authorization_consents().save(consent);

        let result = self.issue_code(
            request,
            response,
            registered_client,
            &redirect_uri,
            Some(state),
            principal_name,
            &approved,
            pending
                .attribute(CODE_CHALLENGE_ATTRIBUTE)
                .map(|challenge| {
                    (
                        challenge.to_string(),
                        pending
                            .attribute(CODE_CHALLENGE_METHOD_ATTRIBUTE)
                            .map(ToString::to_string),
                    )
                }),
            pending.attribute(NONCE_ATTRIBUTE).map(ToString::to_string),
        );

        // The resource owner answered the request, so the authorization request
        // it asked about cannot be answered again.
        self.context.authorizations().remove(&pending);

        result
    }

    /// Verifies the redirect URI of the request against the registration.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `registered_client` - The client of the request.
    fn redirect_uri(
        &self,
        request: &dyn HttpRequest,
        registered_client: &RegisteredClient,
    ) -> Result<String, OAuth2Error> {
        match form_parameter(request, "redirect_uri") {
            Some(redirect_uri) if registered_client.has_redirect_uri(&redirect_uri) => {
                Ok(redirect_uri)
            }
            Some(_) => Err(OAuth2Error::invalid_request(
                "The redirect_uri is not registered for the client",
            )),
            None => {
                let redirect_uris = registered_client.redirect_uris();
                if let [redirect_uri] = redirect_uris {
                    Ok(redirect_uri.clone())
                } else {
                    Err(OAuth2Error::invalid_request(
                        "The redirect_uri parameter is required",
                    ))
                }
            }
        }
    }

    /// Verifies the response type and returns the scopes of the request.
    ///
    /// The scopes of the client are used when the request names none, and every
    /// scope of the request has to be registered for the client.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `registered_client` - The client of the request.
    fn scopes(
        &self,
        request: &dyn HttpRequest,
        registered_client: &RegisteredClient,
    ) -> Result<Vec<String>, OAuth2Error> {
        let response_type =
            form_parameter(request, "response_type").unwrap_or_else(|| "code".to_string());
        if response_type != "code" {
            return Err(OAuth2Error::unsupported_response_type(
                "The authorization endpoint supports the code response type",
            ));
        }

        let requested = form_parameter(request, "scope")
            .map(|scope| {
                scope
                    .split_whitespace()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if requested.is_empty() {
            return Ok(registered_client.scopes().to_vec());
        }

        for scope in &requested {
            if !registered_client.has_scope(scope) {
                return Err(OAuth2Error::invalid_scope(format!(
                    "The {scope} scope is not registered for the client"
                )));
            }
        }

        Ok(requested)
    }

    /// Verifies the proof key of the authorization request.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `registered_client` - The client of the request.
    fn proof_key(
        &self,
        request: &dyn HttpRequest,
        registered_client: &RegisteredClient,
    ) -> Result<Option<(String, Option<String>)>, OAuth2Error> {
        let code_challenge = form_parameter(request, "code_challenge");
        let code_challenge_method = form_parameter(request, "code_challenge_method");

        if registered_client.client_settings().require_proof_key() && code_challenge.is_none() {
            return Err(OAuth2Error::invalid_request(
                "The code_challenge parameter is required",
            ));
        }

        if let Some(method) = code_challenge_method.as_deref() {
            if method != "plain" && method != "S256" {
                return Err(OAuth2Error::invalid_request(
                    "The code_challenge_method is neither plain nor S256",
                ));
            }
        }

        Ok(code_challenge.map(|challenge| (challenge, code_challenge_method)))
    }

    /// Issues the authorization code and redirects the client with it.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    /// * `registered_client` - The client of the request.
    /// * `redirect_uri` - The redirect URI of the client.
    /// * `state` - The state of the authorization request.
    /// * `principal_name` - The name of the resource owner.
    /// * `scopes` - The scopes of the authorization.
    /// * `proof_key` - The proof key of the request and its method.
    #[allow(clippy::too_many_arguments)]
    fn issue_code(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        registered_client: &RegisteredClient,
        redirect_uri: &str,
        state: Option<&str>,
        principal_name: &str,
        scopes: &[String],
        proof_key: Option<(String, Option<String>)>,
        nonce: Option<String>,
    ) -> Result<(), FilterError> {
        let code = next_web_core::Uuid::new_v4().to_string();
        let mut authorization = OAuth2Authorization::new(
            next_web_core::Uuid::new_v4().to_string(),
            registered_client.id_value(),
            principal_name,
        )
        .set_grant_type(AuthorizationGrantType::AuthorizationCode)
        .set_attribute(AUTHORIZATION_CODE_ATTRIBUTE, code.clone())
        .set_attribute("authorization_code_issued_at", now().to_string())
        .set_attribute(REDIRECT_URI_ATTRIBUTE, redirect_uri)
        .set_attribute(SCOPES_ATTRIBUTE, scopes.join(" "));
        if let Some(nonce) = nonce.as_ref() {
            authorization = authorization.set_attribute(NONCE_ATTRIBUTE, nonce.clone());
        }
        if let Some((code_challenge, code_challenge_method)) = proof_key {
            authorization = authorization.set_attribute(CODE_CHALLENGE_ATTRIBUTE, code_challenge);
            if let Some(code_challenge_method) = code_challenge_method {
                authorization = authorization
                    .set_attribute(CODE_CHALLENGE_METHOD_ATTRIBUTE, code_challenge_method);
            }
        }
        if let Some(state) = state {
            authorization = authorization.set_attribute(STATE_ATTRIBUTE, state);
        }
        self.context.authorizations().save(authorization);

        tracing::debug!(
            "The authorization code of the {} client is granted to {principal_name}",
            registered_client.client_id()
        );

        let mut parameters = vec![("code", code.as_str())];
        if let Some(state) = state {
            parameters.push(("state", state));
        }
        send_redirect(request, response, redirect_uri, &parameters)
    }

    /// Reports an error to the client of the authorization request.
    ///
    /// # Arguments
    ///
    /// * `request` - The request of the endpoint.
    /// * `response` - The response of the endpoint.
    /// * `redirect_uri` - The redirect URI of the client.
    /// * `error` - The error to report.
    /// * `state` - The state of the authorization request.
    fn redirect_error(
        &self,
        request: &dyn HttpRequest,
        response: &mut dyn HttpResponse,
        redirect_uri: &str,
        error: OAuth2Error,
        state: Option<&str>,
    ) -> Result<(), FilterError> {
        let mut parameters = vec![("error", error.error())];
        if let Some(description) = error.error_description() {
            parameters.push(("error_description", description));
        }
        if let Some(state) = state {
            parameters.push(("state", state));
        }

        send_redirect(request, response, redirect_uri, &parameters)
    }
}

#[async_trait]
impl HttpFilter for AuthorizationEndpointFilter {
    async fn do_filter(
        &self,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
        filter_chain: &dyn HttpFilterChain,
    ) -> Result<(), FilterError> {
        if request.path() != self.context.settings().authorization_endpoint() {
            return filter_chain.do_filter(request, response).await;
        }

        self.authorize(request, response)
    }
}

impl Named for AuthorizationEndpointFilter {
    fn name(&self) -> &str {
        "AuthorizationEndpointFilter"
    }
}

/// Returns the scopes the resource owner approved on the consent page.
///
/// A consent page submits the scopes it was asked for one by one, so every
/// value of the `scope` parameter is a scope the resource owner approved. A
/// page in which nothing was approved submits no scope at all.
///
/// # Arguments
///
/// * `request` - The request of the endpoint.
fn approved_scopes(request: &dyn HttpRequest) -> Vec<String> {
    form_parameter_map(request)
        .into_iter()
        .filter(|(name, _)| name == "scope")
        .flat_map(|(_, value)| {
            value
                .split_whitespace()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}
