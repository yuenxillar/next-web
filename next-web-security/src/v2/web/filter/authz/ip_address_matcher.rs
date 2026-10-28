use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

/// Matches a request based on IP Address or subnet mask matching against the remote
/// address.
///
/// Both IPv6 and IPv4 addresses are supported, but a matcher which is configured with an
/// IPv4 address will never match a request which returns an IPv6 address, and vice-versa.
#[derive(Clone)]
pub struct IpAddressMatcher {
    n_mask_bits: i32,
    required_address: IpAddr,
}

impl IpAddressMatcher {
    /// Takes a specific IP address or a range specified using the IP/Netmask (e.g.
    /// 192.168.1.0/24 or 202.24.0.0/14).
    ///
    /// # Arguments
    ///
    /// * `ip_address` - the address or range of addresses from which the request must come.
    pub fn new(ip_address: &str) -> Result<Self, String> {
        let parts: Vec<&str> = ip_address.split('/').collect();

        let (address_str, mask_bits) = match parts.len() {
            1 => (parts[0], -1),
            2 => {
                let mask = parts[1]
                    .parse::<i32>()
                    .map_err(|e| format!("Invalid mask bits: {}", e))?;
                (parts[0], mask)
            }
            _ => return Err("Invalid IP address format".to_string()),
        };

        let required_address = Self::parse_address(address_str)?;

        Ok(IpAddressMatcher {
            n_mask_bits: mask_bits,
            required_address,
        })
    }

    pub fn matches(&self, address: &str) -> bool {
        let remote_address = match Self::parse_address(address) {
            Ok(addr) => addr,
            Err(_) => return false,
        };

        if !self.same_ip_version(&remote_address) {
            return false;
        }

        if self.n_mask_bits < 0 {
            return remote_address == self.required_address;
        }

        self.matches_with_mask(&remote_address)
    }

    fn same_ip_version(&self, other: &IpAddr) -> bool {
        matches!(
            (&self.required_address, other),
            (IpAddr::V4(_), IpAddr::V4(_)) | (IpAddr::V6(_), IpAddr::V6(_))
        )
    }

    fn matches_with_mask(&self, remote_address: &IpAddr) -> bool {
        match (&self.required_address, remote_address) {
            (IpAddr::V4(req_addr), IpAddr::V4(rem_addr)) => self.matches_ipv4(req_addr, rem_addr),
            (IpAddr::V6(req_addr), IpAddr::V6(rem_addr)) => self.matches_ipv6(req_addr, rem_addr),
            _ => false,
        }
    }

    fn matches_ipv4(&self, req_addr: &Ipv4Addr, rem_addr: &Ipv4Addr) -> bool {
        let req_octets = req_addr.octets();
        let rem_octets = rem_addr.octets();

        self.apply_mask(&req_octets, &rem_octets, 32)
    }

    fn matches_ipv6(&self, req_addr: &Ipv6Addr, rem_addr: &Ipv6Addr) -> bool {
        let req_octets = req_addr.octets();
        let rem_octets = rem_addr.octets();

        self.apply_mask(&req_octets, &rem_octets, 128)
    }

    fn apply_mask(&self, req_addr: &[u8], rem_addr: &[u8], max_bits: u32) -> bool {
        let n_mask_bits = self.n_mask_bits as u32;
        if n_mask_bits > max_bits {
            return false;
        }

        let odd_bits = n_mask_bits % 8;
        let n_mask_bytes = (n_mask_bits / 8) + if odd_bits == 0 { 0 } else { 1 };
        let n_mask_bytes = n_mask_bytes as usize;

        for i in 0..n_mask_bytes {
            let mask = if i == n_mask_bytes - 1 && odd_bits != 0 {
                let final_byte = ((1u16 << odd_bits) - 1) as u8;
                final_byte << (8 - odd_bits)
            } else {
                0xFF
            };

            if (rem_addr[i] & mask) != (req_addr[i] & mask) {
                return false;
            }
        }

        true
    }

    fn parse_address(address: &str) -> Result<IpAddr, String> {
        IpAddr::from_str(address)
            .map_err(|e| format!("Failed to parse address '{}': {}", address, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_ipv4_match() {
        let matcher = IpAddressMatcher::new("192.168.1.1").unwrap();
        assert!(matcher.matches("192.168.1.1"));
        assert!(!matcher.matches("192.168.1.2"));
    }

    #[test]
    fn test_ipv4_subnet_match() {
        let matcher = IpAddressMatcher::new("192.168.1.0/24").unwrap();
        assert!(matcher.matches("192.168.1.1"));
        assert!(matcher.matches("192.168.1.255"));
        assert!(!matcher.matches("192.168.2.1"));
    }

    #[test]
    fn test_exact_ipv6_match() {
        let matcher = IpAddressMatcher::new("::1").unwrap();
        assert!(matcher.matches("::1"));
        assert!(!matcher.matches("::2"));
    }

    #[test]
    fn test_ip_version_mismatch() {
        let matcher = IpAddressMatcher::new("192.168.1.1").unwrap();
        assert!(!matcher.matches("::1"));
    }

    #[test]
    fn test_invalid_address() {
        assert!(IpAddressMatcher::new("invalid").is_err());
        assert!(IpAddressMatcher::new("192.168.1.1/invalid").is_err());
    }
}
