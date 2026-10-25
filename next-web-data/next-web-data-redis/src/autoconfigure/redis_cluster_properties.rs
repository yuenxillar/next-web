use serde::Deserialize;

/// Cluster properties.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RedisClusterProperties {
    /// List of "host:port" pairs to bootstrap from. This represents an "initial" list
    /// of cluster nodes and is required to have at least one entry.
    nodes: Option<Vec<String>>,

    /// Maximum number of redirects to follow when executing commands across the
    /// cluster.
    max_redirects: Option<u32>,
}

impl RedisClusterProperties {
    /// Returns the list of "host:port" pairs to bootstrap from.
    pub fn nodes(&self) -> Option<&[String]> {
        self.nodes.as_deref()
    }

    /// Sets the list of "host:port" pairs to bootstrap from.
    pub fn set_nodes(&mut self, nodes: Option<Vec<String>>) {
        self.nodes = nodes;
    }

    /// Returns the maximum number of redirects to follow when executing commands
    /// across the cluster.
    pub fn max_redirects(&self) -> Option<u32> {
        self.max_redirects
    }

    /// Sets the maximum number of redirects to follow when executing commands
    /// across the cluster.
    pub fn set_max_redirects(&mut self, max_redirects: Option<u32>) {
        self.max_redirects = max_redirects;
    }
}
