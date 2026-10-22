//! Upstream selection: URI parsing, name resolution and load balancing.

pub mod dns_cache;
pub mod load_balancer;
pub mod uri;

use std::net::SocketAddr;

use pingora::upstreams::peer::HttpPeer;

use crate::config::{RouteConfig, UpstreamConfig};
pub use dns_cache::DnsCache;
pub use load_balancer::StaticUpstreams;
pub use uri::{UpstreamScheme, UpstreamUri};

/// A resolved upstream plus the peer settings it needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamSelection {
    /// Address to connect to.
    pub addr: SocketAddr,
    /// SNI / host name sent in the TLS handshake, never empty for `https`.
    pub sni: String,
    /// Value written to the upstream `Host` header.
    pub host_header: String,
    /// Whether the connection uses TLS.
    pub tls: bool,
    /// Connection reuse group, so routes never share sockets.
    pub group_key: u64,
}

/// Selects a backend for a route.
///
/// Phase 1 ships only the static implementation; a registry backed resolver
/// plugs in here without touching the proxy.
pub trait UpstreamResolver: Send + Sync {
    /// Pick a backend for `route_id`, or `None` when nothing is available.
    fn select(&self, route_id: &str, key: &[u8]) -> Option<SocketAddr>;
}

/// Build the pingora peer for a selection, applying route then gateway timeouts.
pub fn build_peer(
    selection: &UpstreamSelection,
    config: &UpstreamConfig,
    route: &RouteConfig,
) -> HttpPeer {
    let mut peer = HttpPeer::new(selection.addr, selection.tls, selection.sni.clone());
    peer.group_key = selection.group_key;

    let tls = &config.tls;
    let insecure = tls.insecure_skip_verify;
    if insecure {
        tracing::warn!(
            route = %route.id,
            "upstream TLS verification is disabled by `insecure_skip_verify`"
        );
    }
    peer.options.verify_cert = tls.verify_cert && !insecure;
    peer.options.verify_hostname = tls.verify_hostname && !insecure;

    let client = &route.metadata.client;
    peer.options.connection_timeout = Some(client.connect_timeout.unwrap_or(config.connect_timeout));
    peer.options.read_timeout = Some(client.read_timeout.unwrap_or(config.read_timeout));
    peer.options.write_timeout = Some(client.write_timeout.unwrap_or(config.write_timeout));
    peer.options.idle_timeout = Some(config.idle_timeout);

    peer
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use super::{build_peer, UpstreamSelection};
    use crate::config::{RouteConfig, RouteMetadata, UpstreamConfig};

    fn route(id: &str, uri: &str, read_timeout: Option<u64>) -> RouteConfig {
        let mut metadata = RouteMetadata::default();
        metadata.client.read_timeout = read_timeout.map(std::time::Duration::from_secs);

        RouteConfig {
            id: id.to_owned(),
            uri: uri.to_owned(),
            order: 0,
            predicates: Vec::new(),
            filters: Vec::new(),
            upstreams: Vec::new(),
            metadata,
        }
    }

    fn selection(tls: bool) -> UpstreamSelection {
        UpstreamSelection {
            addr: "10.0.0.1:443".parse::<SocketAddr>().unwrap(),
            sni: "example.com".to_owned(),
            host_header: "example.com".to_owned(),
            tls,
            group_key: 7,
        }
    }

    #[test]
    fn https_uri_yields_non_empty_sni() {
        let peer = build_peer(
            &selection(true),
            &UpstreamConfig::default(),
            &route("a", "https://example.com", None),
        );

        assert!(peer.is_tls());
        assert_eq!(peer.sni, "example.com");
    }

    #[test]
    fn verify_cert_defaults_to_true_and_insecure_flag_disables_both() {
        let peer = build_peer(
            &selection(true),
            &UpstreamConfig::default(),
            &route("a", "https://example.com", None),
        );
        assert!(peer.options.verify_cert);
        assert!(peer.options.verify_hostname);

        let mut config = UpstreamConfig::default();
        config.tls.insecure_skip_verify = true;
        let peer = build_peer(&selection(true), &config, &route("a", "https://example.com", None));

        assert!(!peer.options.verify_cert);
        assert!(!peer.options.verify_hostname);
    }

    #[test]
    fn route_metadata_overrides_the_gateway_timeouts() {
        let config = UpstreamConfig::default();
        let peer = build_peer(
            &selection(false),
            &config,
            &route("a", "http://example.com", Some(9)),
        );

        assert_eq!(
            peer.options.read_timeout,
            Some(std::time::Duration::from_secs(9))
        );
        assert_eq!(peer.options.connection_timeout, Some(config.connect_timeout));
        assert_eq!(peer.group_key, 7);
    }
}
