use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use axum::BoxError;
use axum::body::Bytes;
use axum::http::{HeaderMap, HeaderName};
use tracing::{error, info};

use crate::web_server::config::client_config::ClientConfig;
use crate::web_server::models::admin_req::{CallbackParam, RegistryParam};
use crate::web_server::models::api_result::XxlApiResult;

pub static EXECUTOR: &'static str = "EXECUTOR";

#[derive(Clone)]
pub struct AdminClient {
    client_config: Arc<ClientConfig>,
    client: reqwest::Client,
    addrs: Vec<String>,
    headers: HashMap<String, String>,
}

impl AdminClient {
    pub fn new(client_config: Arc<ClientConfig>) -> Result<Self, BoxError> {
        let addrs_str = client_config.server_address.as_str();
        if addrs_str.is_empty() {
            return Err("empty admin service address".into());
        }
        let addrs = addrs_str
            .split(",")
            .filter(|&v| !v.is_empty())
            .map(|v| v.to_owned())
            .collect();
        let mut client_builder =
            reqwest::ClientBuilder::new().timeout(std::time::Duration::from_millis(3000));

        #[cfg(feature = "ssl-mode")]
        if client_config.ssl_danger_accept_invalid_certs {
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }
        client_builder = client_builder.timeout(std::time::Duration::from_millis(3000));
        let client = client_builder.build()?;
        let mut headers = HashMap::new();
        if !client_config.access_token.is_empty() {
            headers.insert(
                "XXL-JOB-ACCESS-TOKEN".to_owned(),
                client_config.access_token.clone(),
            );
            headers.insert("Content-Type".to_owned(), "application/json".to_owned());
            headers.insert(
                "User-Agent".to_owned(),
                format!("xxljob-rs/{}", env!("CARGO_PKG_VERSION")),
            );
        }

        Ok(Self {
            client,
            addrs,
            client_config,
            headers,
        })
    }

    pub async fn registry(&self) -> Result<(), BoxError> {
        let address = format!(
            "http://{}:{}",
            self.client_config.ip, self.client_config.port
        );
        let param = RegistryParam {
            registry_group: EXECUTOR.to_string(),
            registry_key: self.client_config.app_name.clone(),
            registry_value: address,
        };
        let body = serde_json::to_vec(&param)?;
        match self.request(body, "registry").await {
            Ok(_) => {
                info!("admin_client|registry success");
                Ok(())
            }
            Err(e) => {
                error!("admin_client|registry error:{}", &e);
                Err(e)
            }
        }
    }

    pub async fn registry_remove(&self) -> Result<(), BoxError> {
        let address = format!(
            "http://{}:{}",
            self.client_config.ip, self.client_config.port
        );
        let param = RegistryParam {
            registry_group: EXECUTOR.to_string(),
            registry_key: self.client_config.app_name.clone(),
            registry_value: address,
        };
        let body = serde_json::to_vec(&param)?;
        match self.request(body, "registryRemove").await {
            Ok(_) => {
                info!("admin_client|registryRemove success");
                Ok(())
            }
            Err(e) => {
                error!("admin_client|registryRemove error:{}", &e);
                Err(e)
            }
        }
    }

    pub async fn callback(&self, params: &Vec<CallbackParam>) -> Result<(), BoxError> {
        let body = serde_json::to_vec(params)?;
        match self.request(body, "callback").await {
            Ok(_) => {
                info!("admin_client|callback success");
                Ok(())
            }
            Err(e) => {
                error!("admin_client|callback error:{}", &e);
                Err(e)
            }
        }
    }

    async fn request(&self, body: Vec<u8>, sub_url: &str) -> Result<(), BoxError> {
        let mut registry_success = false;

        let body = Bytes::from_owner(body);
        for addr in &self.addrs {
            let url = format!("{}/api/{}", addr, &sub_url);

            match self
                .client
                .post(&url)
                .headers({
                    let mut header_map = HeaderMap::new();
                    for (k, v) in self.headers.iter() {
                        let _ = header_map.insert(HeaderName::from_str(&k)?, v.parse()?);
                    }
                    header_map
                })
                .body(body.clone())
                .send()
                .await
            {
                Ok(resp) => {
                    if let Ok(v) = resp.json::<XxlApiResult<String>>().await {
                        if v.is_success() {
                            registry_success = true;
                            break;
                        }
                    }
                }
                Err(err) => {
                    error!("call response error:{},url:{}", err, &url)
                }
            };
        }
        if !registry_success {
            Err("registry failed".into())
        } else {
            Ok(())
        }
    }
}
