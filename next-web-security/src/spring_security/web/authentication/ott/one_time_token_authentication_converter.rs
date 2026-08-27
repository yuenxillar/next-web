use std::any::Any;

use next_web_core::{traits::http::http_request::HttpRequest, util::StringUtils};
use tracing::debug;

use crate::{
    authentication::ott::one_time_token_authentication_token::OneTimeTokenAuthenticationToken,
    core::{Authentication, AuthenticationError},
    web::authentication::authentication_converter::AuthenticationConverter,
};

#[derive(Default, Clone)]
pub struct OneTimeTokenAuthenticationConverter;

impl AuthenticationConverter for OneTimeTokenAuthenticationConverter {
    fn convert(
        &self,
        request: &dyn HttpRequest,
    ) -> Result<Option<Box<dyn Authentication>>, AuthenticationError> {
        let token = request.parameter("token").unwrap_or_default();

        if !StringUtils::has_text(token) {
            debug!("No token found in request");
            return Ok(None);
        }
        Ok(Some(Box::new(
            OneTimeTokenAuthenticationToken::unauthenticated(token),
        )))
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
