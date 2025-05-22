use rudi_dev::{Properties, Singleton};

/// MQTT Client Configuration Properties
/// MQTT客户端配置属性
/// 
/// This struct represents all configurable properties for an MQTT client
/// 这个结构体表示MQTT客户端的所有可配置属性
#[Singleton(default, binds=[Self::into_properties])]
#[Properties(prefix = "next.mqtt")]
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct MQTTClientProperties {
    /// Client identifier (optional)
    /// 客户端标识符(可选)
    client_id: Option<String>,
    
    /// Broker host address (optional, defaults to localhost)
    /// 代理服务器地址(可选，默认为localhost)
    host: Option<String>,
    
    /// Broker port number (optional, defaults to 1883)
    /// 代理服务器端口号(可选，默认为1883)
    port: Option<u16>,
    
    /// Authentication username (optional)
    /// 认证用户名(可选)
    username: Option<String>,
    
    /// Authentication password (optional)
    /// 认证密码(可选)
    password: Option<String>,
    
    /// List of topics to subscribe to (optional)
    /// 要订阅的主题列表(可选)
    topics: Option<Vec<String>>,
    
    /// Keep alive interval in milliseconds (optional)
    /// 保活间隔时间(毫秒，可选)
    keep_alive: Option<u64>,
    
    /// Connection timeout in milliseconds (optional)
    /// 连接超时时间(毫秒，可选)
    connect_timeout: Option<u64>,
    
    /// Clean session flag (optional)
    /// 清除会话标志(可选)
    clean_session: Option<bool>
}

impl MQTTClientProperties {
    /// Returns client ID if set
    /// 返回设置的客户端ID(如果有)
    pub fn client_id(&self) -> Option<&str> {
        self.client_id.as_deref()
    }

    /// Returns broker host address if set
    /// 返回设置的代理服务器地址(如果有)
    pub fn host(&self) -> Option<&str> {
        self.host.as_deref()
    }

    /// Returns broker port number if set
    /// 返回设置的代理服务器端口号(如果有)
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// Returns authentication username if set
    /// 返回设置的认证用户名(如果有)
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Returns authentication password if set
    /// 返回设置的认证密码(如果有)
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// Returns list of topics to subscribe to
    /// 返回要订阅的主题列表
    /// 
    /// If no topics are configured, returns empty vector
    /// 如果没有配置主题，返回空向量
    pub fn topics(&self) -> Vec<String> {
        if let Some(topics) = self.topics.as_ref() {
            return topics.clone();
        }
        vec![]
    }

    /// Returns keep alive interval in milliseconds if set
    /// 返回设置的保活间隔时间(毫秒，如果有)
    pub fn keep_alive(&self) -> Option<u64> {
        self.keep_alive
    }

    /// Returns connection timeout in milliseconds if set
    /// 返回设置的连接超时时间(毫秒，如果有)
    pub fn connect_timeout(&self) -> Option<u64> {
        self.connect_timeout
    }

    /// Returns clean session flag if set
    /// 返回设置的清除会话标志(如果有)
    pub fn clean_session(&self) -> Option<bool> {
        self.clean_session
    }
}