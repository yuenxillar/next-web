use next_web_macros::properties;
use rudi_dev::singleton;

#[singleton(default, binds=[Self::into_properties])]
#[properties(prefix = "next.xxl_job")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct XxlJobProperties {
    pub(crate) server_address: String,
    pub(crate) access_token: Option<String>,
    pub(crate) app_name: Option<String>,
    pub(crate) ip: Option<String>,
    pub(crate) port: Option<u16>,
    pub(crate) log_path: Option<String>,
    pub(crate) log_retention_days: Option<u32>,
    pub(crate) ssl_danger_accept_invalid_certs: Option<bool>,
}

impl XxlJobProperties {
    pub fn server_address(&self) -> &str {
        &self.server_address
    }

    pub fn access_token(&self) -> Option<&str> {
        self.access_token.as_deref()
    }

    pub fn app_name(&self) -> Option<&str> {
        self.app_name.as_deref()
    }

    pub fn ip(&self) -> Option<&str> {
        self.ip.as_deref()
    }

    pub fn port(&self) -> Option<u16> {
        self.port
    }

    pub fn log_path(&self) -> Option<&str> {
        self.log_path.as_deref()
    }

    pub fn log_retention_days(&self) -> Option<u32> {
        self.log_retention_days
    }

    pub fn ssl_danger_accept_invalid_certs(&self) -> Option<bool> {
        self.ssl_danger_accept_invalid_certs
    }

    pub fn set_server_address(&mut self, server_address: String) {
        self.server_address = server_address;
    }

    pub fn set_access_token(&mut self, access_token: String) {
        self.access_token = Some(access_token);
    }

    pub fn set_app_name(&mut self, app_name: String) {
        self.app_name = Some(app_name);
    }

    pub fn set_ip(&mut self, ip: String) {
        self.ip = Some(ip);
    }

    pub fn set_port(&mut self, port: u16) {
        self.port = Some(port);
    }

    pub fn set_log_path(&mut self, log_path: String) {
        self.log_path = Some(log_path);
    }

    pub fn set_log_retention_days(&mut self, log_retention_days: u32) {
        self.log_retention_days = Some(log_retention_days);
    }

    pub fn set_ssl_danger_accept_invalid_certs(&mut self, ssl_danger_accept_invalid_certs: bool) {
        self.ssl_danger_accept_invalid_certs = Some(ssl_danger_accept_invalid_certs);
    }
}
