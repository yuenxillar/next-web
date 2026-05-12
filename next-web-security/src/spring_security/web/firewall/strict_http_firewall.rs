use super::http_firewall::HttpFirewall;

#[derive(Clone)]
pub struct StrictHttpFirewall {}

impl Default for StrictHttpFirewall {
    fn default() -> Self {
        Self {}
    }
}

impl StrictHttpFirewall {}

impl HttpFirewall for StrictHttpFirewall {
    fn get_firewalled_response(
        &self,
        response: axum::response::Response,
    ) -> axum::response::Response {
        response
    }
}
