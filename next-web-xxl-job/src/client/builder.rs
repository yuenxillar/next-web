use tracing::{debug, warn};

use crate::{
    autoconfigure::xxl_job_properties::XxlJobProperties,
    client::xxl_client::XxlClient,
    executor::{admin_server::ServerAccessActor, executor_actor::ExecutorActor},
    utils::{get_available_port, get_local_ip},
    web_server::{config::client_config::ClientConfig, state::XxlJobAppState},
};

use std::{error::Error, sync::Arc};

#[derive(Clone, Debug, Default)]
pub struct XxlClientBuilder {
    server_address: String,
    access_token: Option<String>,
    app_name: Option<String>,
    ip: Option<String>,
    port: Option<u16>,
    log_path: Option<String>,
    log_retention_days: Option<u32>,
    ssl_danger_accept_invalid_certs: Option<bool>,
}

impl XxlClientBuilder {
    pub fn new(server_address: String) -> Self {
        Self {
            server_address,
            ..Default::default()
        }
    }

    /// 设置访问token要与服务端的token一致
    pub fn set_access_token(mut self, access_token: String) -> Self {
        self.access_token = Some(access_token);
        self
    }

    pub fn set_app_name(mut self, app_name: String) -> Self {
        self.app_name = Some(app_name);
        self
    }

    pub fn set_ip(mut self, ip: String) -> Self {
        self.ip = Some(ip);
        self
    }

    pub fn set_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }
    pub fn set_log_path(mut self, log_path: String) -> Self {
        self.log_path = Some(log_path);
        self
    }

    pub fn set_log_retention_days(mut self, log_retention_days: u32) -> Self {
        self.log_retention_days = Some(log_retention_days);
        self
    }

    pub fn set_ssl_danger_accept_invalid_certs(
        mut self,
        ssl_danger_accept_invalid_certs: bool,
    ) -> Self {
        self.ssl_danger_accept_invalid_certs = Some(ssl_danger_accept_invalid_certs);
        self
    }

    pub fn build(self) -> Result<Arc<XxlClient>, Box<dyn Error>> {
        let start_port = 9900;
        let port = Self::get_port(start_port, self.port);
        if port == 0 {
            return Err("no available port".into());
        }
        let client_config = Arc::new(ClientConfig {
            server_address: self.server_address,
            access_token: self.access_token.unwrap_or_default(),
            app_name: self.app_name.unwrap_or("unknown".to_string()),
            ip: self.ip.unwrap_or(get_local_ip()),
            port,
            log_path: self.log_path.unwrap_or_default(),
            log_retention_days: self.log_retention_days.unwrap_or_default(),
            ssl_danger_accept_invalid_certs: self.ssl_danger_accept_invalid_certs.unwrap_or(true),
        });
        if client_config.access_token.is_empty() {
            warn!("api access_token is empty!");
        }
        let client = build_client(client_config)?;
        Ok(client)
    }

    fn get_port(start_port: u16, option_port: Option<u16>) -> u16 {
        let source_port = option_port.unwrap_or_default();
        let port = if source_port == 0 {
            get_available_port(start_port)
        } else {
            get_available_port(source_port)
        };
        if source_port != port {
            debug!("auto use port: {}", port);
        } else {
            debug!("use set port: {}", source_port);
        }
        port
    }
}

impl From<XxlJobProperties> for XxlClientBuilder {
    fn from(value: XxlJobProperties) -> Self {
        XxlClientBuilder {
            server_address: value.server_address,
            access_token: value.access_token,
            app_name: value.app_name,
            ip: value.ip,
            port: value.port,
            log_path: value.log_path,
            log_retention_days: value.log_retention_days,
            ssl_danger_accept_invalid_certs: value.ssl_danger_accept_invalid_certs,
        }
    }
}

fn build_client(client_config: Arc<ClientConfig>) -> Result<Arc<XxlClient>, Box<dyn Error>> {
    let app_state = Arc::new(XxlJobAppState {
        executor_actor: ExecutorActor::new(client_config.clone()),
        server_access_actor: ServerAccessActor::new(client_config.clone()),
        client_config,
    });
    let client = Arc::new(XxlClient::new(app_state));

    Ok(client)
}
