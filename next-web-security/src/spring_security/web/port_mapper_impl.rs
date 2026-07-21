use std::collections::HashMap;

use crate::web::PortMapper;

/// Concrete implementation of `PortMapper` that obtains HTTP:HTTPS pairs from the
/// application context.
///
/// By default the implementation will assume 80:443 and 8080:8443 are HTTP:HTTPS pairs
/// respectively. If different pairs are required, use `set_port_mappings`.
#[derive(Clone)]
pub struct PortMapperImpl {
    https_port_mappings: HashMap<u16, u16>,
}

impl PortMapperImpl {
    /// Returns the translated (u16 -> u16) version of the original port mapping
    /// specified via `set_port_mappings`.
    pub fn get_translated_port_mappings(&self) -> &HashMap<u16, u16> {
        &self.https_port_mappings
    }

    /// Set to override the default HTTP port to HTTPS port mappings of 80:443, and
    /// 8080:8443.
    ///
    /// # Arguments
    ///
    /// * `new_mappings` - A map consisting of String keys and String values, where for
    ///   each entry the key is the string representation of an integer HTTP port number,
    ///   and the value is the string representation of the corresponding integer HTTPS
    ///   port number.
    ///
    /// # Panics
    ///
    /// Panics if input map does not consist of String keys and values, each representing
    /// an integer port number in the range 1-65535 for that mapping, or if the resulting
    /// mappings are empty.
    pub fn set_port_mappings(&mut self, new_mappings: &HashMap<String, String>) {
        assert!(
            !new_mappings.is_empty(),
            "A valid list of HTTPS port mappings must be provided"
        );

        self.https_port_mappings.clear();

        for (http_port_str, https_port_str) in new_mappings.iter() {
            let http_port: u16 = http_port_str
                .parse()
                .expect(&format!("Invalid HTTP port: {}", http_port_str));
            let https_port: u16 = https_port_str
                .parse()
                .expect(&format!("Invalid HTTPS port: {}", https_port_str));

            assert!(
                Self::is_in_port_range(http_port) && Self::is_in_port_range(https_port),
                "one or both ports out of legal range: {}, {}",
                http_port,
                https_port
            );

            self.https_port_mappings.insert(http_port, https_port);
        }

        assert!(
            !self.https_port_mappings.is_empty(),
            "must map at least one port"
        );
    }

    /// Checks if a port number is in the valid range (1-65535).
    fn is_in_port_range(port: u16) -> bool {
        port >= 1 && port <= 65535
    }
}

impl PortMapper for PortMapperImpl {
    fn lookup_http_port(&self, https_port: u16) -> Option<u16> {
        for (http_port, mapped_https_port) in &self.https_port_mappings {
            if *mapped_https_port == https_port {
                return Some(*http_port);
            }
        }
        None
    }

    fn lookup_https_port(&self, http_port: u16) -> Option<u16> {
        self.https_port_mappings.get(&http_port).copied()
    }
}

impl Default for PortMapperImpl {
    fn default() -> Self {
        let mut mapper = Self {
            https_port_mappings: HashMap::new(),
        };
        mapper.https_port_mappings.insert(80, 443);
        mapper.https_port_mappings.insert(8080, 8443);

        mapper
    }
}
