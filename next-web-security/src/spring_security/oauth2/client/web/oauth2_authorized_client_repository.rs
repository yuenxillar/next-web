//! The repository that loads and stores the authorized client of the current
//! request.

use std::sync::Arc;

use next_web_core::traits::http::{http_request::HttpRequest, http_response::HttpResponse};

use crate::{
    core::Authentication,
    oauth2::{OAuth2AuthorizedClient, OAuth2AuthorizedClientService},
};

pub trait OAuth2AuthorizedClientRepository: Send + Sync {
    fn save_authorized_client(
        &self,
        _authorized_client: OAuth2AuthorizedClient,
        _principal: &dyn Authentication,
        _request: &dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) {
    }

    fn load_authorized_client(
        &self,
        _client_registration_id: &str,
        _principal: &dyn Authentication,
        _request: &dyn HttpRequest,
    ) -> Option<OAuth2AuthorizedClient> {
        None
    }

    fn remove_authorized_client(
        &self,
        _client_registration_id: &str,
        _principal: &dyn Authentication,
        _request: &dyn HttpRequest,
        _response: &mut dyn HttpResponse,
    ) {
    }
}

#[derive(Clone)]
pub struct AuthenticatedPrincipalOAuth2AuthorizedClientRepository {
    authorized_client_service: Arc<dyn OAuth2AuthorizedClientService>,
}

impl AuthenticatedPrincipalOAuth2AuthorizedClientRepository {
    pub fn new(authorized_client_service: Arc<dyn OAuth2AuthorizedClientService>) -> Self {
        Self {
            authorized_client_service,
        }
    }

    pub fn authorized_client_service(&self) -> &Arc<dyn OAuth2AuthorizedClientService> {
        &self.authorized_client_service
    }
}

impl OAuth2AuthorizedClientRepository for AuthenticatedPrincipalOAuth2AuthorizedClientRepository {}
