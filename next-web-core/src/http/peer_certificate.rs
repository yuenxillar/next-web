//! The certificate a client presented over the connection of a request.
//!
//! A certificate is not part of an HTTP request: it is agreed on when the
//! connection is made, and only the layer that made the connection knows it. So
//! that layer — the TLS server an application runs on, or the terminator in
//! front of it — inserts a [`PeerCertificate`] into the request extensions, and
//! the security layer reads it from there.
//!
//! Nothing on an HTTP request proves a certificate, so only a transport that
//! validated it may insert one. A request that arrives over any other connection
//! has none, and the X.509 filter authenticates nobody.

/// The certificate a client presented over the connection of a request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerCertificate {
    encoded: Vec<u8>,
    subject: Option<String>,
}

impl PeerCertificate {
    /// Creates the certificate from the bytes of its DER encoding.
    pub fn new(encoded: impl Into<Vec<u8>>) -> Self {
        Self {
            encoded: encoded.into(),
            subject: None,
        }
    }

    /// Creates the certificate from its DER encoding and the subject the
    /// transport already rendered.
    ///
    /// A terminator that reports the subject of the certificate it validated —
    /// a reverse proxy, for instance — can pass it here, so that the subject
    /// does not have to be read out of the encoding.
    pub fn with_subject(encoded: impl Into<Vec<u8>>, subject: impl Into<String>) -> Self {
        Self {
            encoded: encoded.into(),
            subject: Some(subject.into()),
        }
    }

    /// Returns the DER encoding of the certificate.
    pub fn encoded(&self) -> &[u8] {
        &self.encoded
    }

    /// Returns the subject of the certificate as the transport rendered it, when
    /// it rendered one.
    pub fn subject(&self) -> Option<&str> {
        self.subject.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_encoding_of_a_certificate_is_kept() {
        let certificate = PeerCertificate::new(vec![0x30, 0x82]);

        assert_eq!(certificate.encoded(), &[0x30, 0x82]);
        assert_eq!(certificate.subject(), None, "a transport may render no subject");
    }

    #[test]
    fn the_subject_a_transport_rendered_is_kept() {
        let certificate = PeerCertificate::with_subject(vec![0x30], "CN=Alice, O=Example");

        assert_eq!(certificate.subject(), Some("CN=Alice, O=Example"));
    }
}
