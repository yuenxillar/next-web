use std::collections::HashMap;

/// Maps HTTP ports to HTTPS ports and vice versa for redirects.
/// Used by `HttpsRedirectFilter` to determine the correct port when redirecting.
pub trait PortMapper: Send + Sync {
    /// Look up the HTTPS port for a given HTTP port.
    fn lookup_https_port(&self, http_port: u16) -> Option<u16>;

    /// Look up the HTTP port for a given HTTPS port.
    fn lookup_http_port(&self, https_port: u16) -> Option<u16>;
}

/// Default implementation of `PortMapper`.
/// Stores HTTP-to-HTTPS port mappings.
///
/// Default mappings:
/// - 8080 → 8443
/// - 80 → 443
#[derive(Clone, Default)]
pub struct PortMapperImpl {
    https_port_map: HashMap<u16, u16>,
}

impl PortMapperImpl {
    pub fn new() -> Self {
        let mut mapper = Self {
            https_port_map: HashMap::new(),
        };
        // Default mappings
        mapper.https_port_map.insert(8080, 8443);
        mapper.https_port_map.insert(80, 443);
        mapper
    }

    /// Map an HTTP port to an HTTPS port.
    pub fn http(mut self, http_port: u16) -> PortMappingBuilder {
        PortMappingBuilder {
            mapper: self,
            http_port,
        }
    }

    fn add_mapping(&mut self, http_port: u16, https_port: u16) {
        self.https_port_map.insert(http_port, https_port);
    }
}

/// Builder for port mappings (e.g. `.http(8080).maps_to(8443)`).
pub struct PortMappingBuilder {
    mapper: PortMapperImpl,
    http_port: u16,
}

impl PortMappingBuilder {
    pub fn maps_to(mut self, https_port: u16) -> PortMapperImpl {
        self.mapper.add_mapping(self.http_port, https_port);
        self.mapper
    }
}

impl PortMapper for PortMapperImpl {
    fn lookup_https_port(&self, http_port: u16) -> Option<u16> {
        self.https_port_map.get(&http_port).copied()
    }

    fn lookup_http_port(&self, https_port: u16) -> Option<u16> {
        self.https_port_map
            .iter()
            .find(|(_, &v)| v == https_port)
            .map(|(&k, _)| k)
    }
}
