//! Resolving the real client address behind a trusted proxy.
//!
//! `X-Forwarded-For` is only consulted when the direct peer is one of the
//! gateway's `trusted_proxies`: with an empty list (the default) the header is
//! ignored entirely, so a client cannot spoof its own address.

use std::net::IpAddr;
use std::sync::OnceLock;

use ipnetwork::IpNetwork;

static TRUSTED_PROXIES: OnceLock<Vec<IpNetwork>> = OnceLock::new();

/// Install the trusted proxy list once, at startup.
pub(crate) fn set_trusted_proxies(networks: Vec<IpNetwork>) {
    if TRUSTED_PROXIES.set(networks).is_err() {
        tracing::warn!("trusted proxies were already installed, ignoring the new list");
    }
}

/// The installed trusted proxy list. Empty means "trust nobody".
pub(crate) fn trusted_proxies() -> &'static [IpNetwork] {
    TRUSTED_PROXIES.get().map(Vec::as_slice).unwrap_or_default()
}

/// Resolve the client address from the direct peer and `X-Forwarded-For`.
///
/// Returns the direct peer when it is not trusted, otherwise the rightmost
/// forwarded entry that is not itself a trusted proxy. `None` means the client
/// cannot be determined, which callers must treat as a rejection.
pub(crate) fn resolve_client(
    direct: IpAddr,
    forwarded: Option<&str>,
    trusted: &[IpNetwork],
) -> Option<IpAddr> {
    let direct = normalize_ip(direct);

    if !trusted.iter().any(|network| network.contains(direct)) {
        return Some(direct);
    }

    let forwarded = forwarded?;

    // Walk right to left, skipping the proxies we already know about.
    forwarded.split(',').rev().find_map(|entry| {
        let ip: IpAddr = entry.trim().parse().ok()?;
        let ip = normalize_ip(ip);
        (!trusted.iter().any(|network| network.contains(ip))).then_some(ip)
    })
}

/// Collapse an IPv4-mapped IPv6 address onto its IPv4 form.
pub(crate) fn normalize_ip(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(IpAddr::V6(v6), IpAddr::V4),
        IpAddr::V4(v4) => IpAddr::V4(v4),
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use ipnetwork::IpNetwork;

    use super::{normalize_ip, resolve_client};

    fn networks(entries: &[&str]) -> Vec<IpNetwork> {
        entries.iter().map(|entry| entry.parse().unwrap()).collect()
    }

    #[test]
    fn untrusted_peer_ignores_the_forwarded_header() {
        let direct: IpAddr = "203.0.113.9".parse().unwrap();
        let trusted = networks(&["10.0.0.0/8"]);

        assert_eq!(
            resolve_client(direct, Some("1.2.3.4"), &trusted),
            Some(direct)
        );
    }

    #[test]
    fn trusted_peer_uses_the_rightmost_untrusted_entry() {
        let direct: IpAddr = "10.0.0.1".parse().unwrap();
        let trusted = networks(&["10.0.0.0/8"]);

        assert_eq!(
            resolve_client(direct, Some("1.2.3.4, 10.0.0.7"), &trusted),
            Some("1.2.3.4".parse().unwrap())
        );
    }

    #[test]
    fn a_trusted_peer_without_the_header_resolves_to_nothing() {
        let direct: IpAddr = "10.0.0.1".parse().unwrap();
        let trusted = networks(&["10.0.0.0/8"]);

        assert_eq!(resolve_client(direct, None, &trusted), None);
    }

    #[test]
    fn no_trusted_proxies_configured_never_reads_the_header() {
        let direct: IpAddr = "10.0.0.1".parse().unwrap();

        assert_eq!(resolve_client(direct, Some("1.2.3.4"), &[]), Some(direct));
    }

    #[test]
    fn ipv4_mapped_ipv6_is_normalised() {
        let peer: IpAddr = "::ffff:10.0.0.5".parse().unwrap();
        assert_eq!(normalize_ip(peer), "10.0.0.5".parse::<IpAddr>().unwrap());
    }
}
