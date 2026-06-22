use next_web_core::{traits::http::http_request::HttpRequest, util::StringUtils};
use tracing::debug;

use crate::{
    authentication::ott::one_time_token_authentication_token::OneTimeTokenAuthenticationToken,
    core::Authentication, web::authentication::authentication_converter::AuthenticationConverter,
};

#[derive(Default)]
pub struct OneTimeTokenAuthenticationConverter;

impl AuthenticationConverter for OneTimeTokenAuthenticationConverter {
    fn convert(&self, request: &dyn HttpRequest) -> Option<Box<dyn Authentication>> {
        let token = request.parameter("token")?;

        if !StringUtils::has_text(token) {
            debug!("No token found in request");
            return None;
        }
        return Some(Box::new(OneTimeTokenAuthenticationToken::unauthenticated(
            token,
        )));
    }
}
