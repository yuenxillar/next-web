//! The repository that keeps the authorization request between the request
//! that starts the authorization code grant and the redirect back to the
//! application.

use next_web_core::{
    any::AnyValue,
   http::{HttpRequest, HttpResponse},
};

use crate::oauth2::OAuth2AuthorizationRequest;

pub trait AuthorizationRequestRepository: Send + Sync {
    fn load_authorization_request(
        &self,
        request: &dyn HttpRequest,
    ) -> Option<OAuth2AuthorizationRequest>;

    fn save_authorization_request(
        &self,
        authorization_request: Option<OAuth2AuthorizationRequest>,
        request: &mut dyn HttpRequest,
        response: &mut dyn HttpResponse,
    );

    fn remove_authorization_request(
        &self,
        request: &mut dyn HttpRequest,
    ) -> Option<OAuth2AuthorizationRequest>;
}

#[derive(Clone)]
pub struct HttpSessionOAuth2AuthorizationRequestRepository {
    session_attribute_name: String,
}

impl HttpSessionOAuth2AuthorizationRequestRepository {
    pub const DEFAULT_AUTHORIZATION_REQUEST_ATTR_NAME: &'static str =
        "next_web_security.oauth2.authorization_request";

    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for HttpSessionOAuth2AuthorizationRequestRepository {
    fn default() -> Self {
        Self {
            session_attribute_name: Self::DEFAULT_AUTHORIZATION_REQUEST_ATTR_NAME.to_string(),
        }
    }
}

impl AuthorizationRequestRepository for HttpSessionOAuth2AuthorizationRequestRepository {
    fn load_authorization_request(
        &self,
        request: &dyn HttpRequest,
    ) -> Option<OAuth2AuthorizationRequest> {
        request.session().and_then(|session| {
            session
                .attribute(&self.session_attribute_name)
                .and_then(|value| value.as_ref_value::<OAuth2AuthorizationRequest>())
                .cloned()
        })
    }

    fn save_authorization_request(
        &self,
        authorization_request: Option<OAuth2AuthorizationRequest>,
        request: &mut dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) {
        match authorization_request {
            Some(authorization_request) => {
                if let Some(session) = request.session_mut(true) {
                    session.set_attribute(
                        &self.session_attribute_name,
                        AnyValue::BoxedValue(Box::new(authorization_request)),
                    );
                }
            }
            None => {
                if let Some(session) = request.session_mut(false) {
                    session.remove_attribute(&self.session_attribute_name);
                }
            }
        }
    }

    fn remove_authorization_request(
        &self,
        request: &mut dyn HttpRequest,
    ) -> Option<OAuth2AuthorizationRequest> {
        let session = request.session_mut(false)?;
        let authorization_request = session
            .attribute(&self.session_attribute_name)
            .and_then(|value| value.as_ref_value::<OAuth2AuthorizationRequest>())
            .cloned();
        if authorization_request.is_some() {
            session.remove_attribute(&self.session_attribute_name);
        }
        authorization_request
    }
}
