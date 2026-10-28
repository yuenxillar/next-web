use std::fmt;

/// A lightweight representation of an X.509 client certificate.
///
/// Only the information required by the pre-authentication filter is retained: the
/// subject distinguished name and, optionally, the encoded certificate bytes.
#[derive(Clone, PartialEq, Eq)]
pub struct X509Certificate {
    subject_dn: String,
    encoded: Option<Vec<u8>>,
}

impl X509Certificate {
    /// Creates a certificate from its subject distinguished name.
    pub fn new(subject_dn: impl Into<String>) -> Self {
        Self {
            subject_dn: subject_dn.into(),
            encoded: None,
        }
    }

    /// Creates a certificate from its subject distinguished name and encoded bytes.
    pub fn with_encoded(subject_dn: impl Into<String>, encoded: Vec<u8>) -> Self {
        Self {
            subject_dn: subject_dn.into(),
            encoded: Some(encoded),
        }
    }

    /// Returns the subject distinguished name of this certificate.
    pub fn subject_dn(&self) -> &str {
        &self.subject_dn
    }

    /// Returns the encoded certificate bytes, if available.
    pub fn encoded(&self) -> Option<&[u8]> {
        self.encoded.as_deref()
    }
}

impl fmt::Debug for X509Certificate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("X509Certificate")
            .field("subject_dn", &self.subject_dn)
            .field("encoded_len", &self.encoded.as_ref().map(Vec::len))
            .finish()
    }
}

impl fmt::Display for X509Certificate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.subject_dn)
    }
}

#[cfg(test)]
mod tests {
    use super::X509Certificate;

    #[test]
    fn exposes_subject_dn() {
        let certificate = X509Certificate::new("CN=Luke Taylor");
        assert_eq!(certificate.subject_dn(), "CN=Luke Taylor");
        assert!(certificate.encoded().is_none());
    }

    #[test]
    fn exposes_encoded_bytes() {
        let certificate = X509Certificate::with_encoded("CN=Duke", vec![1, 2, 3]);
        assert_eq!(certificate.encoded(), Some([1, 2, 3].as_slice()));
    }
}
