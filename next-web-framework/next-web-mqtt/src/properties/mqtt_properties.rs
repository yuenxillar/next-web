use rudi_dev::{Properties, Singleton};

/// MQTT Client Configuration Properties
///
/// MQTT客户端配置属性
#[Singleton(default, binds=[Self::into_properties])]
#[Properties(prefix = "next.mqtt")]
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
pub struct MQTTClientProperties {
    /// Client identifier (optional)
    ///
    /// 客户端标识符(可选)
    client_id: Option<String>,

    /// Broker host address (optional, defaults to localhost)
    ///
    /// 代理服务器地址(可选，默认为localhost)
    host: Option<String>,

    /// Broker port number (optional, defaults to 1883)
    ///
    /// 代理服务器端口号(可选，默认为1883)
    port: Option<u16>,

    /// Authentication username (optional)
    ///
    /// 认证用户名(可选)
    username: Option<String>,

    /// Authentication password (optional)
    ///
    /// 认证密码(可选)
    password: Option<String>,

    /// List of topics to subscribe to (optional)
    /// We perform character segmentation to determine the message quality you need to subscribe to the topic with
    /// Example: ["topic1/#", "topic2/+/test"] or ["topic1/#:0", "topic2/+/test:1"]
    ///
    /// 要订阅的主题列表(可选)
    /// 我们执行字符分割以确定您订阅主题所需的消息质量
    topics: Option<Vec<Topic>>,

    /// Keep alive interval in milliseconds (optional)
    ///
    /// 保活间隔时间(毫秒，可选)
    keep_alive: Option<u64>,

    /// Connection timeout in seconds (optional)
    ///
    /// 连接超时时间(秒，可选)
    connect_timeout: Option<u64>,

    /// Clean session flag (optional)
    ///
    /// 清除会话标志(可选)
    clean_session: Option<bool>,
}

impl MQTTClientProperties {
    /// Returns client ID
    /// 返回设置的客户端ID
    pub fn client_id(&self) -> Option<&str> {
        self.client_id.as_deref()
    }

    /// Returns broker host address
    ///
    /// 返回设置的代理服务器地址
    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    /// Returns broker port number
    ///
    /// 返回设置的代理服务器端口号
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// Returns authentication username
    ///
    /// 返回设置的认证用户名(如果有)
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Returns authentication password
    ///
    /// 返回设置的认证密码(如果有)
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// Returns list of topics to subscribe to
    /// If no topics are configured, returns empty vector
    ///
    /// 返回要订阅的主题列表
    /// 如果没有配置主题，返回空向量
    pub fn topics(&self) -> Vec<Topic> {
        if let Some(topics) = self.topics.as_ref() {
            return topics.clone();
        }
        vec![]
    }

    /// Returns keep alive interval in milliseconds
    ///
    /// 返回设置的保活间隔时间(毫秒)
    pub fn keep_alive(&self) -> Option<u64> {
        self.keep_alive
    }

    /// Returns connection timeout in milliseconds
    ///
    /// 返回设置的连接超时时间(秒)
    pub fn connect_timeout(&self) -> Option<u64> {
        self.connect_timeout
    }

    /// Returns clean session flag
    ///
    /// 返回设置的清除会话标志
    pub fn clean_session(&self) -> Option<bool> {
        self.clean_session
    }

    pub fn set_client_id(&mut self, client_id: impl ToString) {
        self.client_id = Some(client_id.to_string());
    }

    pub fn set_host(&mut self, host: impl ToString) {
        self.host = Some(host.to_string());
    }

    pub fn set_port(&mut self, port: Option<u16>) {
        self.port = port;
    }

    pub fn set_username(&mut self, username: impl ToString) {
        self.username = Some(username.to_string());
    }

    pub fn set_password(&mut self, password: impl ToString) {
        self.password = Some(password.to_string());
    }

    pub fn set_topics(&mut self, topics: impl IntoIterator<Item = Topic>) {
        self.topics = Some(topics.into_iter().collect());
    }

    pub fn set_keep_alive(&mut self, keep_alive: u64) {
        self.keep_alive = Some(keep_alive);
    }

    pub fn set_connect_timeout(&mut self, connect_timeout: u64) {
        self.connect_timeout = Some(connect_timeout);
    }

    pub fn set_clean_session(&mut self, clean_session: bool) {
        self.clean_session = Some(clean_session);
    }
}


#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]

pub struct Topic {
    pub topic:  String,
    pub qos:    Option<u8>,
}