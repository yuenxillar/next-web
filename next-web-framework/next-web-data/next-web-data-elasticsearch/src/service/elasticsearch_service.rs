use std::
    ops::{Deref, DerefMut}
;

use elasticsearch::{Elasticsearch, auth::Credentials, http::transport::Transport};
use next_web_core::interface::{service::Service, singleton::Singleton};

use crate::properties::elasticsearch_properties::ElasticsearchClientProperties;

/// Elasticsearch 服务结构体
///
/// Elasticsearch service
#[derive(Clone)]
pub struct ElasticsearchService {
    /// Elasticsearch客户端配置属性
    ///
    /// Elasticsearch client configuration properties
    properties: ElasticsearchClientProperties,
    /// Elasticsearch客户端实例
    ///
    ///  Elasticsearch client instance
    client: Elasticsearch,
}
impl Singleton  for ElasticsearchService {}
impl Service    for ElasticsearchService {}

impl ElasticsearchService {
    /// 创建新的Elasticsearch服务实例
    ///  Create new Elasticsearch service instance
    pub fn new(properties: ElasticsearchClientProperties) -> Self {
        let client = Self::build_client(&properties);

        Self {
            properties,
            client,
        }
    }

    /// 构建Elasticsearch客户端
    /// Build Elasticsearch client
    fn build_client(config: &ElasticsearchClientProperties) -> Elasticsearch {
        let username = config.username().unwrap_or_default();
        let password = config.password().unwrap_or_default();

        let url = format!(
            "http://{}:{}",
            config.host().unwrap_or("localhost"),
            config.port().unwrap_or(9200)
        );

        let credentials = if username.is_empty() && password.is_empty() {
            None
        } else {
            Some(Credentials::Basic(username.into(), password.into()))
        };

        let transport = Transport::single_node(url.as_ref()).expect("Invalid elasticsearch URL.");
        credentials.map(|auth| transport.set_auth(auth));

        let client = Elasticsearch::new(transport);

        client
    }

    /// 获取Elasticsearch客户端引用
    ///  Get Elasticsearch client reference
    pub fn get_client(&self) -> &Elasticsearch {
        &self.client
    }

    /// 获取配置属性
    ///  Get configuration properties
    pub fn properties(&self) -> &ElasticsearchClientProperties {
        &self.properties
    }
}

impl Deref for ElasticsearchService {
    type Target = Elasticsearch;
    /// 解引用为Elasticsearch客户端
    ///  Dereference to Elasticsearch client
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for ElasticsearchService {
    /// 可变解引用为Elasticsearch客户端
    ///  Mutable dereference to Elasticsearch client
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}
