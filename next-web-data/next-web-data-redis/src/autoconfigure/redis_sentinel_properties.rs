use serde::Deserialize;

/// Redis sentinel properties.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RedisSentinelProperties {
    /// Name of the Redis server.
    master: Option<String>,

    /// List of "host:port" pairs.
    nodes: Option<Vec<String>>,

    /// Login username for authenticating with sentinel(s).
    username: Option<String>,

    /// Password for authenticating with sentinel(s).
    password: Option<String>,
}

impl RedisSentinelProperties {
    /// Returns the name of the Redis server.
    pub fn master(&self) -> Option<&str> {
        self.master.as_deref()
    }

    /// Sets the name of the Redis server.
    pub fn set_master(&mut self, master: Option<String>) {
        self.master = master;
    }

    /// Returns the list of "host:port" pairs.
    pub fn nodes(&self) -> Option<&[String]> {
        self.nodes.as_deref()
    }

    /// Sets the list of "host:port" pairs.
    pub fn set_nodes(&mut self, nodes: Option<Vec<String>>) {
        self.nodes = nodes;
    }

    /// Returns the login username for authenticating with sentinel(s).
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Sets the login username for authenticating with sentinel(s).
    pub fn set_username(&mut self, username: Option<String>) {
        self.username = username;
    }

    /// Returns the password for authenticating with sentinel(s).
    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    /// Sets the password for authenticating with sentinel(s).
    pub fn set_password(&mut self, password: Option<String>) {
        self.password = password;
    }
}
