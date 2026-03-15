#[derive(Clone, Debug, Default)]
pub struct ClientConfig {
    pub server_address: String,
    pub access_token: String,
    pub app_name: String,
    pub ip: String,
    pub port: u16,
    pub log_path: String,
    pub log_retention_days: u32,
    pub ssl_danger_accept_invalid_certs: bool,
}

impl ClientConfig {
    pub fn get_http_addr(&self) -> String {
        format!("0.0.0.0:{}", &self.port)
    }
}
